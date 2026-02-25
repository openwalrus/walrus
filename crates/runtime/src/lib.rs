//! Walrus runtime: the top-level orchestrator.
//!
//! The [`Runtime`] is the entry point for the agent framework. It holds
//! the LLM provider, agent configurations, tool handlers, and manages
//! chat sessions.
//!
//! # Example
//!
//! ```rust,ignore
//! use walrus_core::Agent;
//! use walrus_runtime::{Runtime, Provider};
//! use llm::{General, Message};
//!
//! let provider = Provider::new("deepseek-chat", Client::new(), &key)?;
//! let mut runtime = Runtime::new(General::default(), provider);
//! runtime.add_agent(Agent::new("assistant").system_prompt("You are helpful."));
//! let mut chat = runtime.chat("assistant")?;
//! let response = runtime.send(&mut chat, Message::user("hello")).await?;
//! ```

pub use provider::Provider;
pub use team::{build_team, extract_input, worker_tool};

use agent::{Agent, Chat};
use anyhow::Result;
use compact_str::CompactString;
use futures_core::Stream;
use futures_util::StreamExt;
use llm::{
    Config, FinishReason, General, LLM, Message, Response, Role, StreamChunk, Tool, ToolCall,
    ToolChoice, estimate_tokens,
};
use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

mod provider;
pub mod team;

const MAX_TOOL_CALLS: usize = 16;

/// A type-erased async tool handler.
pub type Handler =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = String> + Send>> + Send + Sync>;

/// A compaction function that trims message history.
pub type Compactor = Arc<dyn Fn(Vec<Message>) -> Vec<Message> + Send + Sync>;

/// The walrus runtime — top-level orchestrator.
///
/// Holds the LLM provider, agent configurations, tool handlers,
/// compactors, and internal chat sessions.
pub struct Runtime {
    provider: Provider,
    config: General,
    tools: BTreeMap<CompactString, (Tool, Handler)>,
    compactors: BTreeMap<CompactString, Compactor>,
    agents: BTreeMap<CompactString, Agent>,
    sessions: BTreeMap<CompactString, Chat>,
}

impl Runtime {
    /// Create a new runtime with the given config and provider.
    pub fn new(config: General, provider: Provider) -> Self {
        Self {
            provider,
            config,
            tools: BTreeMap::new(),
            compactors: BTreeMap::new(),
            agents: BTreeMap::new(),
            sessions: BTreeMap::new(),
        }
    }

    /// Register an agent.
    pub fn add_agent(&mut self, agent: Agent) {
        self.agents.insert(agent.name.clone(), agent);
    }

    /// Get a registered agent by name.
    pub fn agent(&self, name: &str) -> Option<&Agent> {
        self.agents.get(name)
    }

    /// Register a tool with its handler.
    pub fn register<F, Fut>(&mut self, tool: Tool, handler: F)
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = String> + Send + 'static,
    {
        let name = tool.name.clone();
        let handler: Handler = Arc::new(move |args| Box::pin(handler(args)));
        self.tools.insert(name, (tool, handler));
    }

    /// Set a compaction function for a specific agent.
    pub fn set_compactor<F>(&mut self, agent: &str, compactor: F)
    where
        F: Fn(Vec<Message>) -> Vec<Message> + Send + Sync + 'static,
    {
        self.compactors
            .insert(CompactString::from(agent), Arc::new(compactor));
    }

    /// Create a new chat session for the named agent.
    pub fn chat(&self, agent: &str) -> Result<Chat> {
        if !self.agents.contains_key(agent) {
            anyhow::bail!("agent '{agent}' not registered");
        }
        Ok(Chat::new(agent))
    }

    /// Context window limit for the current provider/model.
    pub fn context_limit(&self) -> usize {
        self.provider.context_limit(&self.config)
    }

    /// Estimate current token usage for a chat session.
    pub fn estimate_tokens(&self, chat: &Chat) -> usize {
        let system_tokens = self
            .agents
            .get(chat.agent_name())
            .map(|a| (a.system_prompt.len() / 4).max(1))
            .unwrap_or(0);
        system_tokens + estimate_tokens(&chat.messages)
    }

    /// Check if a chat session is approaching the context limit.
    pub fn needs_compaction(&self, chat: &Chat) -> bool {
        let usage = self.estimate_tokens(chat);
        let limit = self.context_limit();
        usage > (limit * 4 / 5)
    }

    /// Resolve tool schemas for the given tool names.
    fn resolve(&self, names: &[CompactString]) -> Vec<Tool> {
        names
            .iter()
            .filter_map(|name| self.tools.get(name.as_str()).map(|(tool, _)| tool.clone()))
            .collect()
    }

    /// Dispatch tool calls and collect results as tool messages.
    async fn dispatch(&self, calls: &[ToolCall]) -> Vec<Message> {
        let mut results = Vec::with_capacity(calls.len());
        for call in calls {
            let output = if let Some((_, handler)) = self.tools.get(call.function.name.as_str()) {
                handler(call.function.arguments.clone()).await
            } else {
                format!("function {} not available", call.function.name)
            };
            results.push(Message::tool(output, call.id.clone()));
        }
        results
    }

    /// Apply compaction for the given agent.
    fn compact(&self, agent: &str, messages: Vec<Message>) -> Vec<Message> {
        match self.compactors.get(agent) {
            Some(compactor) => compactor(messages),
            None => messages,
        }
    }

    /// Build the message list for an API request.
    fn api_messages(&self, chat: &Chat) -> Vec<Message> {
        let agent = match self.agents.get(chat.agent_name()) {
            Some(a) => a,
            None => return chat.messages.clone(),
        };

        let mut messages = chat.messages.clone();

        if messages.first().map(|m| m.role) != Some(Role::System) {
            messages.insert(0, Message::system(&agent.system_prompt));
        }

        self.compact(&agent.name, messages)
            .into_iter()
            .map(|mut m| {
                if m.tool_calls.is_empty() {
                    m.reasoning_content = String::new();
                }
                m
            })
            .collect()
    }

    /// Build a config with the given tools and tool choice.
    fn build_config(&self, tools: Vec<Tool>, tool_choice: ToolChoice) -> General {
        self.config
            .clone()
            .with_tools(tools)
            .with_tool_choice(tool_choice)
    }

    /// Send a message through a chat session (non-streaming).
    pub async fn send(&self, chat: &mut Chat, message: Message) -> Result<Response> {
        let agent = self
            .agents
            .get(chat.agent_name())
            .ok_or_else(|| anyhow::anyhow!("agent '{}' not registered", chat.agent_name))?;
        let tools = self.resolve(&agent.tools);
        let mut tool_choice = ToolChoice::Auto;
        chat.messages.push(message);

        for _ in 0..MAX_TOOL_CALLS {
            let messages = self.api_messages(chat);
            let cfg = self.build_config(tools.clone(), tool_choice.clone());
            let response = self.provider.send(&cfg, &messages).await?;
            let Some(message) = response.message() else {
                return Ok(response);
            };

            if message.tool_calls.is_empty() {
                chat.messages.push(message);
                return Ok(response);
            }

            let result = self.dispatch(&message.tool_calls).await;
            chat.messages.push(message);
            chat.messages.extend(result);
            tool_choice = ToolChoice::None;
        }

        anyhow::bail!("max tool calls reached");
    }

    /// Stream a message through a chat session.
    pub fn stream<'a>(
        &'a self,
        chat: &'a mut Chat,
        message: Message,
    ) -> impl Stream<Item = Result<StreamChunk>> + 'a {
        let agent = self.agents.get(chat.agent_name()).cloned();
        let tools = agent
            .as_ref()
            .map(|a| self.resolve(&a.tools))
            .unwrap_or_default();

        async_stream::try_stream! {
            if agent.is_none() {
                Err(anyhow::anyhow!("agent '{}' not registered", chat.agent_name))?;
            }

            chat.messages.push(message);
            let mut tool_choice = ToolChoice::Auto;

            for _ in 0..MAX_TOOL_CALLS {
                let messages = self.api_messages(chat);
                let cfg = self.build_config(tools.clone(), tool_choice.clone());
                let mut builder = Message::builder(Role::Assistant);

                let inner = self.provider.stream(cfg, &messages, self.config.usage);
                futures_util::pin_mut!(inner);

                while let Some(result) = inner.next().await {
                    let chunk = match result {
                        Ok(chunk) => chunk,
                        Err(e) => {
                            tracing::error!("Error in LLM stream: {:?}", e);
                            Err(e)?
                        }
                    };

                    let reason = chunk.reason().cloned();

                    if builder.accept(&chunk) {
                        yield chunk;
                    }

                    if let Some(reason) = reason {
                        match reason {
                            FinishReason::Stop => return,
                            FinishReason::ToolCalls => break,
                            reason => Err(anyhow::anyhow!("unexpected finish reason: {reason:?}"))?,
                        }
                    }
                }

                let message = builder.build();
                if message.tool_calls.is_empty() {
                    chat.messages.push(message);
                    break;
                }

                let result = self.dispatch(&message.tool_calls).await;
                chat.messages.push(message);
                chat.messages.extend(result);
                tool_choice = ToolChoice::None;
            }
        }
    }

    /// Convenience: send to a named agent using an internal session.
    pub async fn send_to(&mut self, agent: &str, message: Message) -> Result<Response> {
        let key = CompactString::from(agent);
        let mut chat = self
            .sessions
            .remove(&key)
            .unwrap_or_else(|| Chat::new(agent));
        let result = self.send(&mut chat, message).await;
        self.sessions.insert(key, chat);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm::{FunctionCall, LLM};

    fn test_provider() -> Provider {
        // We can't create a real provider without an API key,
        // so we test tool registry/dispatch separately.
        Provider::DeepSeek(deepseek::DeepSeek::new(llm::Client::new(), "test-key").unwrap())
    }

    fn echo_tool() -> Tool {
        Tool {
            name: "echo".into(),
            description: "Echoes the input".into(),
            parameters: schemars::schema_for!(String),
            strict: false,
        }
    }

    #[test]
    fn resolve_returns_registered_tools() {
        let mut rt = Runtime::new(General::default(), test_provider());
        rt.register(echo_tool(), |args| async move { args });
        let tools = rt.resolve(&["echo".into()]);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
    }

    #[test]
    fn resolve_skips_unknown() {
        let rt = Runtime::new(General::default(), test_provider());
        let tools = rt.resolve(&["missing".into()]);
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn dispatch_calls_handler() {
        let mut rt = Runtime::new(General::default(), test_provider());
        rt.register(echo_tool(), |args| async move { format!("got: {args}") });

        let calls = vec![ToolCall {
            id: "call_1".into(),
            index: 0,
            call_type: "function".into(),
            function: FunctionCall {
                name: "echo".into(),
                arguments: "hello".into(),
            },
        }];

        let results = rt.dispatch(&calls).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "got: hello");
        assert_eq!(results[0].tool_call_id, "call_1");
    }

    #[tokio::test]
    async fn dispatch_unknown_tool() {
        let rt = Runtime::new(General::default(), test_provider());
        let calls = vec![ToolCall {
            id: "call_1".into(),
            index: 0,
            call_type: "function".into(),
            function: FunctionCall {
                name: "missing".into(),
                arguments: "".into(),
            },
        }];

        let results = rt.dispatch(&calls).await;
        assert!(results[0].content.contains("not available"));
    }

    #[test]
    fn compactor_applied() {
        let mut rt = Runtime::new(General::default(), test_provider());
        rt.set_compactor("test", |msgs| msgs.into_iter().take(1).collect());

        let msgs = vec![Message::user("first"), Message::user("second")];
        let compacted = rt.compact("test", msgs);
        assert_eq!(compacted.len(), 1);
        assert_eq!(compacted[0].content, "first");
    }

    #[test]
    fn no_compactor_passthrough() {
        let rt = Runtime::new(General::default(), test_provider());
        let msgs = vec![Message::user("hello")];
        let result = rt.compact("any", msgs.clone());
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn chat_requires_registered_agent() {
        let rt = Runtime::new(General::default(), test_provider());
        assert!(rt.chat("unknown").is_err());
    }

    #[test]
    fn chat_succeeds_with_agent() {
        let mut rt = Runtime::new(General::default(), test_provider());
        rt.add_agent(Agent::new("test").system_prompt("hello"));
        let chat = rt.chat("test").unwrap();
        assert_eq!(chat.agent_name(), "test");
        assert!(chat.messages.is_empty());
    }

    #[test]
    fn context_limit_default() {
        let rt = Runtime::new(General::default(), test_provider());
        assert_eq!(rt.context_limit(), 64_000);
    }

    #[test]
    fn context_limit_override() {
        let mut config = General::default();
        config.context_limit = Some(128_000);
        let rt = Runtime::new(config, test_provider());
        assert_eq!(rt.context_limit(), 128_000);
    }

    #[test]
    fn estimate_tokens_counts() {
        let mut rt = Runtime::new(General::default(), test_provider());
        rt.add_agent(Agent::new("test").system_prompt("You are helpful."));
        let mut chat = rt.chat("test").unwrap();
        chat.messages.push(Message::user("hello world"));
        let tokens = rt.estimate_tokens(&chat);
        assert!(tokens > 0);
    }
}
