//! Agent configuration.
//!
//! An [`Agent`] is pure config — name, system prompt, and tool names.
//! Tool handlers live in the [`Runtime`](crate::Runtime).

/// An agent configuration.
///
/// Agents are portable configs: they describe *what* an agent does
/// but not *how* tool calls are dispatched. The [`Runtime`](crate::Runtime)
/// holds the actual tool handlers.
#[derive(Debug, Clone, Default)]
pub struct Agent {
    /// Agent identifier (used for tool registration in teams).
    pub name: String,
    /// Human-readable description (shown as tool description in teams).
    pub description: String,
    /// System prompt sent before each LLM request.
    pub system_prompt: String,
    /// Names of tools this agent can use (resolved by Runtime).
    pub tools: Vec<String>,
}

impl Agent {
    /// Create a new agent with the given name.
    pub fn new(name: impl Into<String>) -> Self {
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
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a tool by name.
    pub fn tool(mut self, name: impl Into<String>) -> Self {
        self.tools.push(name.into());
        self
    }
}
