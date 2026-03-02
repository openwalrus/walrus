//! Listener trait for external message sources.
//!
//! Listeners produce a stream of incoming messages routed to agents.
//! Channel implementations (Telegram, webhooks, etc.) provide concrete listeners.

use compact_str::CompactString;
use futures_core::Stream;

/// An incoming message from an external source.
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    /// The source identifier (channel name, user ID, etc.).
    pub source: CompactString,
    /// The target agent name.
    pub agent: CompactString,
    /// The message content.
    pub content: String,
}

/// External message source that produces a stream of incoming messages.
///
/// Implementations wrap channel connectors (Telegram, webhooks, etc.)
/// and yield messages as they arrive. Runtime dispatches each message
/// to the target agent.
pub trait Listener: Send + Sync {
    /// Start listening and yield incoming messages.
    fn listen(&self) -> impl Stream<Item = IncomingMessage> + Send;
}
