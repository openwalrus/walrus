//! Pluggable conversation memory backend.
//!
//! Implementations store and retrieve messages for a conversation.
//! The [`Chat`](crate::Chat) uses this instead of owning messages directly,
//! enabling persistent conversation across sessions.

use crate::Message;
use anyhow::Result;

/// Conversation memory backend.
///
/// Load returns the message history, append stores new messages
/// produced during a turn.
pub trait Memory: Clone + Send + Sync {
    /// Load the conversation history.
    fn load(&self) -> impl Future<Output = Result<Vec<Message>>> + Send;

    /// Append new messages produced during a turn.
    fn append(&mut self, messages: &[Message]) -> impl Future<Output = Result<()>> + Send;
}

/// Simple in-memory storage (default, backward-compatible behavior).
#[derive(Clone, Default)]
pub struct InMemory {
    pub messages: Vec<Message>,
}

impl Memory for InMemory {
    async fn load(&self) -> Result<Vec<Message>> {
        Ok(self.messages.clone())
    }

    async fn append(&mut self, messages: &[Message]) -> Result<()> {
        self.messages.extend_from_slice(messages);
        Ok(())
    }
}
