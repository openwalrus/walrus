//! Chat session — agent name + message history.

use compact_str::CompactString;
use llm::Message;

/// A chat session: agent name + conversation messages.
pub struct Chat {
    /// The agent name for this session.
    pub agent_name: CompactString,
    /// Conversation messages.
    pub messages: Vec<Message>,
}

impl Chat {
    /// Create a new chat session.
    pub fn new(agent_name: impl Into<CompactString>) -> Self {
        Self {
            agent_name: agent_name.into(),
            messages: Vec::new(),
        }
    }

    /// Get the agent name for this session.
    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }
}
