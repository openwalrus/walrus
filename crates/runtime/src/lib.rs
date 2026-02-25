//! Walrus runtime: the top-level orchestrator.
//!
//! The [`Runtime`] is the entry point for the agent framework. It holds
//! the LLM provider, agent configurations, tool handlers, and manages
//! chat sessions.
//!
//! # Example
//!
//! ```rust,ignore
//! use walrus_runtime::prelude::*;
//!
//! let provider = Provider::deepseek(&key)?;
//! let mut runtime = Runtime::new(General::default(), provider, InMemory::new());
//! runtime.add_agent(Agent::new("assistant").system_prompt("You are helpful."));
//! let mut chat = runtime.chat("assistant")?;
//! let response = runtime.send(&mut chat, Message::user("hello")).await?;
//! ```

pub use agent::{Agent, InMemory, Memory, Skill, SkillTier};
pub use chat::Chat;
pub use hook::{DEFAULT_COMPACT_PROMPT, DEFAULT_FLUSH_PROMPT, Hook};
pub use llm::{Client, General, Message, Response, Role, StreamChunk, Tool};
pub use mcp::McpBridge;
pub use provider::Provider;
pub use skills::{SkillRegistry, parse_skill_md};
pub use team::{build_team, extract_input, worker_tool};

use anyhow::Result;
use compact_str::CompactString;
use futures_core::Stream;
use futures_util::StreamExt;
use llm::{Config, FinishReason, LLM, ToolCall, ToolChoice, estimate_tokens};
use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

mod chat;
pub mod hook;
pub mod mcp;
mod provider;
pub mod skills;
pub mod team;

/// Re-exports of the most commonly used types.
pub mod prelude {
    pub use crate::{
        Agent, Chat, General, Hook, InMemory, Message, Provider, Response, Role, Runtime,
        SkillRegistry, StreamChunk, Tool,
    };
}

const MAX_TOOL_CALLS: usize = 16;

/// A type-erased async tool handler.
pub type Handler =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = String> + Send>> + Send + Sync>;

/// The walrus runtime — top-level orchestrator.
///
/// Generic over `H: Hook` for type-level configuration of memory and
/// compaction prompts. Defaults to [`InMemory`] when no persistence is needed.
pub struct Runtime<H: Hook> {
    provider: Provider,
    config: General,
    memory: Arc<H::Memory>,
    skills: Option<SkillRegistry>,
    mcp: Option<Arc<McpBridge>>,
    tools: BTreeMap<CompactString, (Tool, Handler)>,
    agents: BTreeMap<CompactString, Agent>,
    sessions: BTreeMap<CompactString, Chat>,
}

impl<H: Hook + 'static> Runtime<H> {
    /// Create a new runtime with the given config, provider, and memory.
    pub fn new(config: General, provider: Provider, memory: H::Memory) -> Self {
        let memory = Arc::new(memory);
        let mut rt = Self {
            provider,
            config,
            memory: Arc::clone(&memory),
            skills: None,
            mcp: None,
            tools: BTreeMap::new(),
            agents: BTreeMap::new(),
            sessions: BTreeMap::new(),
        };

        // Auto-register the "remember" tool (DD#23).
        let mem = memory;
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "key": { "type": "string", "description": "Memory key" },
                "value": { "type": "string", "description": "Value to remember" }
            },
            "required": ["key", "value"]
        });
        let tool = Tool {
            name: "remember".into(),
            description: "Store a key-value pair in memory.".into(),
            parameters: serde_json::from_value(schema).unwrap(),
            strict: false,
        };
        rt.register(tool, move |args| {
            let mem = Arc::clone(&mem);
            async move {
                let parsed: serde_json::Value = match serde_json::from_str(&args) {
                    Ok(v) => v,
                    Err(e) => return format!("invalid arguments: {e}"),
                };
                let key = parsed["key"].as_str().unwrap_or("");
                let value = parsed["value"].as_str().unwrap_or("");
                match mem.store(key.to_owned(), value.to_owned()).await {
                    Ok(()) => format!("remembered: {key}"),
                    Err(e) => format!("failed to store: {e}"),
                }
            }
        });

        rt
    }

    /// Set the skill registry for this runtime (builder-style).
    pub fn with_skills(mut self, registry: SkillRegistry) -> Self {
        self.skills = Some(registry);
        self
    }

    /// Set the skill registry for this runtime (mutable setter).
    pub fn set_skills(&mut self, registry: SkillRegistry) {
        self.skills = Some(registry);
    }

    /// Connect an MCP bridge to this runtime.
    pub fn connect_mcp(&mut self, bridge: McpBridge) {
        self.mcp = Some(Arc::new(bridge));
    }

    /// Get a reference to the MCP bridge, if connected.
    pub fn mcp_bridge(&self) -> Option<&McpBridge> {
        self.mcp.as_deref()
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
    ///
    /// Supports glob prefixes (DD#21): a name ending in `*` expands against
    /// all registered tool names by prefix match. No-match globs are logged.
    pub fn resolve(&self, names: &[CompactString]) -> Vec<Tool> {
        let mut resolved = Vec::new();
        for name in names {
            if let Some(prefix) = name.strip_suffix('*') {
                let mut matched = false;
                for (tool_name, (tool, _)) in &self.tools {
                    if tool_name.starts_with(prefix) {
                        resolved.push(tool.clone());
                        matched = true;
                    }
                }
                if !matched {
                    tracing::warn!("glob pattern '{name}' matched no registered tools");
                }
            } else if let Some((tool, _)) = self.tools.get(name.as_str()) {
                resolved.push(tool.clone());
            }
        }
        resolved
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

    /// Build the message list for an API request.
    ///
    /// Injects the agent system prompt (enriched with memory) if not already present.
    async fn api_messages(&self, chat: &Chat) -> Vec<Message> {
        let agent = match self.agents.get(chat.agent_name()) {
            Some(a) => a,
            None => return chat.messages.clone(),
        };

        let mut messages = chat.messages.clone();

        // Build system prompt: base + memory context + skill bodies.
        let mut system_prompt = agent.system_prompt.clone();
        let last_user_msg = messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| m.content.as_str())
            .unwrap_or("");
        let memory_context = self.memory.compile_relevant(last_user_msg).await;
        if !memory_context.is_empty() {
            system_prompt = format!("{system_prompt}\n\n{memory_context}");
        }

        // Inject matched skill bodies.
        if let Some(registry) = &self.skills {
            let matched = registry.find_by_tags(&agent.skill_tags);
            for skill in matched {
                if !skill.body.is_empty() {
                    system_prompt = format!("{system_prompt}\n\n{}", skill.body);
                }
            }
        }

        if messages.first().map(|m| m.role) != Some(Role::System) {
            messages.insert(0, Message::system(&system_prompt));
        }

        messages
            .into_iter()
            .map(|mut m| {
                if m.tool_calls.is_empty() {
                    m.reasoning_content = String::new();
                }
                m
            })
            .collect()
    }

    /// Automatic compaction: flush memory then summarize conversation.
    ///
    /// Called at the start of `send()` and each loop iteration in `stream()`.
    /// Uses `H::flush()` and `H::compact()` as static calls (no Hook instance).
    async fn maybe_compact(&self, chat: &mut Chat) {
        if !self.needs_compaction(chat) {
            return;
        }

        // 1. Memory flush: extract durable facts via "remember" tool.
        let flush_prompt = H::flush();
        if !flush_prompt.is_empty() {
            let remember_tool = self.resolve(&["remember".into()]);
            let mut flush_messages = chat.messages.clone();
            flush_messages.insert(0, Message::system(flush_prompt));
            let cfg = self
                .config
                .clone()
                .with_tools(remember_tool)
                .with_tool_choice(ToolChoice::Auto);
            match self.provider.send(&cfg, &flush_messages).await {
                Ok(response) => {
                    if let Some(msg) = response.message()
                        && !msg.tool_calls.is_empty()
                    {
                        self.dispatch(&msg.tool_calls).await;
                    }
                }
                Err(e) => tracing::warn!("memory flush failed during compaction: {e}"),
            }
        }

        // 2. Summarize conversation history.
        let compact_prompt = H::compact();
        if !compact_prompt.is_empty() {
            let mut summary_messages = chat.messages.clone();
            summary_messages.insert(0, Message::system(compact_prompt));
            let cfg = self
                .config
                .clone()
                .with_tools(vec![])
                .with_tool_choice(ToolChoice::None);
            match self.provider.send(&cfg, &summary_messages).await {
                Ok(response) => {
                    let summary = response.content().cloned().unwrap_or_default();
                    chat.messages.clear();
                    chat.messages.push(Message::assistant(&summary, None, None));
                    chat.compaction_count += 1;
                }
                Err(e) => tracing::warn!("compaction summarization failed: {e}"),
            }
        }
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
        self.maybe_compact(chat).await;

        for _ in 0..MAX_TOOL_CALLS {
            let messages = self.api_messages(chat).await;
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
            self.maybe_compact(chat).await;
            let mut tool_choice = ToolChoice::Auto;

            for _ in 0..MAX_TOOL_CALLS {
                let messages = self.api_messages(chat).await;
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

    /// Get a reference to the memory backend.
    pub fn memory(&self) -> &H::Memory {
        &self.memory
    }

    /// Get a clone of the memory Arc (for team delegation).
    pub fn memory_arc(&self) -> Arc<H::Memory> {
        Arc::clone(&self.memory)
    }

    /// Get a reference to the provider.
    pub fn provider(&self) -> &Provider {
        &self.provider
    }

    /// Get a reference to the general config.
    pub fn config(&self) -> &General {
        &self.config
    }

    /// Resolve tool handlers for the given tool names.
    ///
    /// Like [`resolve`] but returns both tool schemas and handlers.
    /// Supports glob prefixes (names ending in `*`).
    pub fn resolve_handlers(&self, names: &[CompactString]) -> BTreeMap<CompactString, Handler> {
        let mut resolved = BTreeMap::new();
        for name in names {
            if let Some(prefix) = name.strip_suffix('*') {
                for (tool_name, (_, handler)) in &self.tools {
                    if tool_name.starts_with(prefix) {
                        resolved.insert(tool_name.clone(), Arc::clone(handler));
                    }
                }
            } else if let Some((_, handler)) = self.tools.get(name.as_str()) {
                resolved.insert(name.clone(), Arc::clone(handler));
            }
        }
        resolved
    }
}
