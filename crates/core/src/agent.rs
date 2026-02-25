//! Agent configuration.
//!
//! An [`Agent`] is pure config — name, system prompt, and tool names.
//! Tool handlers live in the runtime.

use compact_str::CompactString;
use smallvec::SmallVec;

/// An agent configuration.
///
/// Agents are portable configs: they describe *what* an agent does
/// but not *how* tool calls are dispatched. The runtime holds the
/// actual tool handlers.
#[derive(Debug, Clone, Default)]
pub struct Agent {
    /// Agent identifier (used for tool registration in teams).
    pub name: CompactString,
    /// Human-readable description (shown as tool description in teams).
    pub description: CompactString,
    /// System prompt sent before each LLM request.
    pub system_prompt: String,
    /// Names of tools this agent can use (resolved by Runtime).
    pub tools: SmallVec<[CompactString; 8]>,
    /// Skill tags for matching agent capabilities to skills.
    pub skill_tags: SmallVec<[CompactString; 4]>,
}

impl Agent {
    /// Create a new agent with the given name.
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
}