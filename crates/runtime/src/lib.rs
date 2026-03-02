//! Walrus runtime: the top-level orchestrator.
//!
//! The [`Runtime`] holds the model provider, agent configurations, tool handlers,
//! and manages sessions. Agents are created as ephemeral execution units per
//! request, driven via `Agent::run()` and `Agent::run_stream()`.

pub use dispatcher::RuntimeDispatcher;
pub use hook::Hook;
pub use listener::{IncomingMessage, Listener};
pub use loader::{CronEntry, load_agents_dir, load_cron_dir, parse_agent_md, parse_cron_md};
pub use mcp::McpBridge;
pub use memory::{InMemory, Memory, NoEmbedder};
pub use skills::{Skill, SkillRegistry, SkillTier, parse_skill_md};
pub use team::{build_team, extract_input, worker_tool};
pub use wcore::AgentConfig;
pub use wcore::model::{Message, Request, Response, Role, StreamChunk, Tool};

use anyhow::Result;
use compact_str::CompactString;
use futures_core::Stream;
use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};
use tokio::sync::RwLock;
use wcore::AgentEvent;
use wcore::model::Model;

pub mod dispatcher;
pub mod hook;
pub mod listener;
pub mod loader;
pub mod mcp;
pub mod skills;
pub mod team;

/// Re-exports of the most commonly used types.
pub mod prelude {
    pub use crate::{
        AgentConfig, Hook, InMemory, Message, Request, Response, Role, Runtime, SkillRegistry,
        StreamChunk, Tool,
    };
}

/// A type-erased async tool handler.
pub type Handler =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = String> + Send>> + Send + Sync>;

/// Private session state, keyed by agent name in the sessions map.
struct Session {
    messages: Vec<Message>,
}

impl Session {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

/// The walrus runtime — top-level orchestrator.
///
/// Generic over `H: Hook` for type-level configuration of the model provider.
/// Stores agent configurations, tool handlers, skill registry, and MCP bridge.
/// Creates ephemeral [`wcore::Agent`] instances per request.
pub struct Runtime<H: Hook> {
    provider: H::Model,
    request: Request,
    memory: Arc<H::Memory>,
    skills: Option<Arc<SkillRegistry>>,
    mcp: Option<Arc<McpBridge>>,
    tools: BTreeMap<CompactString, (Tool, Handler)>,
    agents: BTreeMap<CompactString, AgentConfig>,
    sessions: RwLock<BTreeMap<CompactString, Session>>,
}

impl<H: Hook + 'static> Runtime<H> {
    /// Create a new runtime with the given request template, provider, and memory.
    pub fn new(request: Request, provider: H::Model, memory: H::Memory) -> Self {
        let memory = Arc::new(memory);
        Self {
            provider,
            request,
            memory,
            skills: None,
            mcp: None,
            tools: BTreeMap::new(),
            agents: BTreeMap::new(),
            sessions: RwLock::new(BTreeMap::new()),
        }
    }

    /// Set the skill registry for this runtime (builder-style).
    pub fn with_skills(mut self, registry: SkillRegistry) -> Self {
        self.skills = Some(Arc::new(registry));
        self
    }

    /// Set the skill registry for this runtime (mutable setter).
    pub fn set_skills(&mut self, registry: SkillRegistry) {
        self.skills = Some(Arc::new(registry));
    }

    /// Connect an MCP bridge to this runtime.
    pub fn connect_mcp(&mut self, bridge: McpBridge) {
        self.mcp = Some(Arc::new(bridge));
    }

    /// Get a reference to the MCP bridge, if connected.
    pub fn mcp_bridge(&self) -> Option<&McpBridge> {
        self.mcp.as_deref()
    }

    /// Register all MCP tools from the connected bridge into the tool registry.
    pub async fn register_mcp_tools(&mut self) -> Result<()> {
        let bridge = self
            .mcp
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no MCP bridge connected"))?;
        let mcp_tools = bridge.tools().await;
        for tool in mcp_tools {
            let name = tool.name.clone();
            let bridge = Arc::clone(bridge);
            self.tools.insert(
                name.clone(),
                (
                    tool,
                    Arc::new(move |args: String| {
                        let bridge = Arc::clone(&bridge);
                        let name = name.clone();
                        Box::pin(async move { bridge.call(&name, &args).await })
                    }),
                ),
            );
        }
        Ok(())
    }

    /// Register an agent configuration.
    pub fn add_agent(&mut self, agent: AgentConfig) {
        self.agents.insert(agent.name.clone(), agent);
    }

    /// Get a registered agent config by name.
    pub fn agent(&self, name: &str) -> Option<&AgentConfig> {
        self.agents.get(name)
    }

    /// Iterate over all registered agent configs in alphabetical order.
    pub fn agents(&self) -> impl Iterator<Item = &AgentConfig> {
        self.agents.values()
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

    /// Context window limit for a specific model.
    pub fn context_limit(&self, model: &str) -> usize {
        self.provider.context_limit(model)
    }

    /// Clear the session for a named agent, resetting conversation history.
    pub async fn clear_session(&self, agent: &str) {
        self.sessions.write().await.remove(agent);
    }

    /// Resolve tool schemas and handlers for the given tool names.
    ///
    /// Supports glob prefixes: a name ending in `*` expands against
    /// all registered tool names by prefix match.
    pub fn resolve_tools(&self, names: &[CompactString]) -> Vec<(Tool, Handler)> {
        let mut resolved = Vec::new();
        for name in names {
            if let Some(prefix) = name.strip_suffix('*') {
                let mut matched = false;
                for (tool_name, (tool, handler)) in &self.tools {
                    if tool_name.starts_with(prefix) {
                        resolved.push((tool.clone(), Arc::clone(handler)));
                        matched = true;
                    }
                }
                if !matched {
                    tracing::warn!("glob pattern '{name}' matched no registered tools");
                }
            } else if let Some((tool, handler)) = self.tools.get(name.as_str()) {
                resolved.push((tool.clone(), Arc::clone(handler)));
            }
        }
        resolved
    }

    /// Get a reference to the memory backend.
    pub fn memory(&self) -> &H::Memory {
        &self.memory
    }

    /// Get a clone of the memory Arc.
    pub fn memory_arc(&self) -> Arc<H::Memory> {
        Arc::clone(&self.memory)
    }

    /// Get a reference to the model provider.
    pub fn provider(&self) -> &H::Model {
        &self.provider
    }

    /// Get a reference to the request template.
    pub fn request(&self) -> &Request {
        &self.request
    }

    /// Get a shared reference to the skill registry, if one is set.
    pub fn skills(&self) -> Option<&Arc<SkillRegistry>> {
        self.skills.as_ref()
    }

    /// Build a RuntimeDispatcher for the given agent config.
    ///
    /// Resolves the agent's tool names against the registry and includes
    /// the MCP bridge if connected.
    pub fn build_dispatcher(&self, agent_config: &AgentConfig) -> RuntimeDispatcher {
        let resolved = self.resolve_tools(&agent_config.tools);
        let mut tools = Vec::with_capacity(resolved.len());
        let mut handlers = BTreeMap::new();
        for (tool, handler) in resolved {
            handlers.insert(tool.name.clone(), handler);
            tools.push(tool);
        }
        RuntimeDispatcher::new(tools, handlers, self.mcp.clone())
    }

    /// Build a system prompt enriched with skills for the given agent config.
    fn build_system_prompt(&self, agent_config: &AgentConfig) -> String {
        let mut prompt = agent_config.system_prompt.clone();
        if let Some(registry) = &self.skills {
            for skill in registry.find_by_tags(&agent_config.skill_tags) {
                if !skill.body.is_empty() {
                    prompt.push_str("\n\n");
                    prompt.push_str(&skill.body);
                }
            }
        }
        prompt
    }

    /// Create an ephemeral Agent from config with a fresh event channel.
    ///
    /// Returns the Agent and a receiver for draining events.
    fn create_agent(
        &self,
        agent_config: &AgentConfig,
        history: Vec<Message>,
    ) -> (wcore::Agent, tokio::sync::mpsc::Receiver<AgentEvent>) {
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let enriched_prompt = self.build_system_prompt(agent_config);
        let mut config = agent_config.clone();
        config.system_prompt = enriched_prompt;
        let mut agent = wcore::AgentBuilder::new(tx).config(config).build();
        for msg in history {
            agent.push_message(msg);
        }
        (agent, rx)
    }

    /// Send a message to a named agent using Agent.run().
    ///
    /// Creates an ephemeral Agent, runs it with a RuntimeDispatcher,
    /// and returns the final response. Events are emitted via Hook::on_event().
    pub async fn send_to(&self, agent: &str, message: Message) -> Result<Response> {
        let agent_config = self
            .agents
            .get(agent)
            .ok_or_else(|| anyhow::anyhow!("agent '{agent}' not registered"))?
            .clone();
        let key = CompactString::from(agent);

        let mut session = {
            self.sessions
                .write()
                .await
                .remove(&key)
                .unwrap_or_else(Session::new)
        };
        session.messages.push(message);

        let dispatcher = self.build_dispatcher(&agent_config);
        let (mut agent_instance, mut rx) = self.create_agent(&agent_config, session.messages);

        // Spawn event drain task.
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                H::on_event(&event);
            }
        });

        let agent_response = agent_instance.run(&self.provider, &dispatcher).await;
        let messages = agent_instance.messages().to_vec();
        self.sessions
            .write()
            .await
            .insert(key, Session { messages });

        agent_response
            .steps
            .last()
            .map(|s| s.response.clone())
            .ok_or_else(|| anyhow::anyhow!("agent produced no response"))
    }

    /// Stream events from a named agent using Agent.run_stream().
    ///
    /// Creates an ephemeral Agent and yields AgentEvents as a stream.
    pub fn stream_to<'a>(
        &'a self,
        agent: &'a str,
        message: Message,
    ) -> impl Stream<Item = AgentEvent> + 'a {
        async_stream::stream! {
            let agent_config = match self.agents.get(agent) {
                Some(c) => c.clone(),
                None => {
                    yield AgentEvent::Done(wcore::AgentResponse {
                        steps: vec![],
                        final_response: Some(format!("agent '{agent}' not registered")),
                        iterations: 0,
                        stop_reason: wcore::AgentStopReason::Error(
                            format!("agent '{agent}' not registered"),
                        ),
                    });
                    return;
                }
            };
            let key = CompactString::from(agent);

            let mut session = {
                self.sessions.write().await.remove(&key).unwrap_or_else(Session::new)
            };
            session.messages.push(message);

            let dispatcher = self.build_dispatcher(&agent_config);
            let (mut agent_instance, _rx) = self.create_agent(&agent_config, session.messages);

            {
                let stream = agent_instance.run_stream(&self.provider, &dispatcher);
                futures_util::pin_mut!(stream);

                while let Some(event) = futures_util::StreamExt::next(&mut stream).await {
                    H::on_event(&event);
                    yield event;
                }
            }

            let messages = agent_instance.messages().to_vec();
            self.sessions.write().await.insert(key, Session { messages });
        }
    }
}
