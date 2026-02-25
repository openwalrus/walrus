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

pub use chat::Chat;
pub use provider::Provider;
pub use team::{build_team, extract_input, worker_tool};

use agent::Agent;
use anyhow::Result;
use compact_str::CompactString;
use futures_core::Stream;
use futures_util::StreamExt;
use llm::{
    Config, FinishReason, General, LLM, Message, Response, Role, StreamChunk, Tool, ToolCall,
    ToolChoice, estimate_tokens,
};
use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

mod chat;
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
    pub fn resolve(&self, names: &[CompactString]) -> Vec<Tool> {
        names
            .iter()
            .filter_map(|name| self.tools.get(name.as_str()).map(|(tool, _)| tool.clone()))
            .collect()
    }

    /// Dispatch tool calls and collect results as tool messages.
    pub async fn dispatch(&self, calls: &[ToolCall]) -> Vec<Message> {
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
    pub fn compact(&self, agent: &str, messages: Vec<Message>) -> Vec<Message> {
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