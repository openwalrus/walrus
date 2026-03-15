//! Fluent builder for constructing an [`Agent`].

use crate::{
    agent::{Agent, CompactHook, config::AgentConfig, tool::ToolSender},
    model::{Model, Tool},
};
use std::sync::Arc;

/// Fluent builder for [`Agent<M>`].
///
/// Requires a model at construction. Use [`AgentConfig`] builder methods
/// for field configuration, then pass it via [`AgentBuilder::config`].
pub struct AgentBuilder<M: Model> {
    config: AgentConfig,
    model: M,
    tools: Vec<Tool>,
    tool_tx: Option<ToolSender>,
    compact_hook: Option<Arc<dyn CompactHook>>,
}

impl<M: Model> AgentBuilder<M> {
    /// Create a new builder with the given model.
    pub fn new(model: M) -> Self {
        Self {
            config: AgentConfig::default(),
            model,
            tools: Vec::new(),
            tool_tx: None,
            compact_hook: None,
        }
    }

    /// Set the full config, replacing all fields.
    pub fn config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the tool schemas advertised to the LLM.
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
        self
    }

    /// Set the tool sender for dispatching tool calls.
    pub fn tool_tx(mut self, tx: ToolSender) -> Self {
        self.tool_tx = Some(tx);
        self
    }

    /// Set the compact hook for auto-compaction.
    pub fn compact_hook(mut self, hook: Arc<dyn CompactHook>) -> Self {
        self.compact_hook = Some(hook);
        self
    }

    /// Build the [`Agent`].
    pub fn build(self) -> Agent<M> {
        Agent {
            config: self.config,
            model: self.model,
            tools: self.tools,
            tool_tx: self.tool_tx,
            compact_hook: self.compact_hook,
        }
    }
}
