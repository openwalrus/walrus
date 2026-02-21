//! Chat session — agent name + message history.

use llm::Message;

/// A chat session: agent name + conversation messages.
///
/// Created by [`Runtime::chat()`](crate::Runtime::chat). Pass to
/// [`Runtime::send()`](crate::Runtime::send) or
/// [`Runtime::stream()`](crate::Runtime::stream) for orchestration.
pub struct Chat {
    pub(crate) agent_name: String,
    /// Conversation messages.
    pub messages: Vec<Message>,
}

impl Chat {
    /// Create a new chat session.
    pub(crate) fn new(agent_name: String) -> Self {
        Self {
            agent_name,
            messages: Vec::new(),
        }
    }

    /// Get the agent name for this session.
    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }
}
