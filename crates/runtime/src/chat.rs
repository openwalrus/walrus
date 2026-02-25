//! Chat session — agent name + message history.

use compact_str::CompactString;
use llm::Message;

/// A chat session: agent name + conversation messages.
pub struct Chat {
    /// The agent name for this session.
    pub agent_name: CompactString,
    /// Conversation messages.
    pub messages: Vec<Message>,
    /// Number of times this session has been compacted.
    pub compaction_count: usize,
}

impl Chat {
    /// Create a new chat session.
    pub fn new(agent_name: impl Into<CompactString>) -> Self {
        Self {
            agent_name: agent_name.into(),
            messages: Vec::new(),
            compaction_count: 0,
        }
    }

    /// Get the agent name for this session.
    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }

    /// Number of messages in this session.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether this session has no messages.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get the last message, if any.
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Number of times this session has been compacted.
    pub fn compaction_count(&self) -> usize {
        self.compaction_count
    }
}
