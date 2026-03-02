//! Agent configuration.
//!
//! [`AgentConfig`] is a serializable struct holding all agent parameters.
//! Used by [`super::AgentBuilder`] to construct an [`super::Agent`].

use crate::model::ToolChoice;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Default maximum iterations for agent execution.
const DEFAULT_MAX_ITERATIONS: usize = 16;

/// Serializable agent configuration.
///
/// Contains all parameters for an agent: identity, system prompt, model,
/// iteration limits, and tool/skill metadata. The Runtime uses `tools` and
/// `skill_tags` to construct the appropriate Dispatcher for this agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent identifier.
    pub name: CompactString,
    /// Human-readable description.
    pub description: CompactString,
    /// System prompt sent before each LLM request.
    pub system_prompt: String,
    /// Model to use from the registry. None = registry's active/default.
    pub model: Option<CompactString>,
    /// Maximum iterations before stopping.
    pub max_iterations: usize,
    /// Controls which tool the model calls.
    pub tool_choice: ToolChoice,
    /// Names of tools this agent can use (resolved by Runtime into a Dispatcher).
    pub tools: SmallVec<[CompactString; 8]>,
    /// Skill tags for matching agent capabilities.
    pub skill_tags: SmallVec<[CompactString; 4]>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: CompactString::default(),
            description: CompactString::default(),
            system_prompt: String::new(),
            model: None,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            tool_choice: ToolChoice::Auto,
            tools: SmallVec::new(),
            skill_tags: SmallVec::new(),
        }
    }
}

impl AgentConfig {
    /// Create a new config with the given name and defaults for everything else.
    pub fn new(name: impl Into<CompactString>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the system prompt.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<CompactString>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a tool by name.
    pub fn tool(mut self, name: impl Into<CompactString>) -> Self {
        self.tools.push(name.into());
        self
    }

    /// Add a skill tag.
    pub fn skill_tag(mut self, tag: impl Into<CompactString>) -> Self {
        self.skill_tags.push(tag.into());
        self
    }

    /// Set the model to use from the registry.
    pub fn model(mut self, name: impl Into<CompactString>) -> Self {
        self.model = Some(name.into());
        self
    }
}
