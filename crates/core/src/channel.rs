//! Channel trait and types for platform integrations.
//!
//! Channels connect agents to messaging platforms (Telegram, etc.).
//! Each channel provides a stream of events and a way to send messages.

use std::future::Future;
use compact_str::CompactString;
use futures_core::Stream;

/// Messaging platform identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Platform {
    /// Telegram messaging platform.
    Telegram,
}

/// A message received from or sent to a channel.
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    /// Platform this message belongs to.
    pub platform: Platform,
    /// Channel/chat identifier on the platform.
    pub channel_id: CompactString,
    /// Sender identifier on the platform.
    pub sender_id: CompactString,
    /// Message text content.
    pub content: String,
    /// Attached files or media.
    pub attachments: Vec<Attachment>,
    /// ID of the message being replied to, if any.
    pub reply_to: Option<CompactString>,
    /// Unix timestamp when the message was created.
    pub timestamp: u64,
}

/// A file or media attachment.
#[derive(Debug, Clone)]
pub struct Attachment {
    /// Type of attachment.
    pub kind: AttachmentKind,
    /// URL or path to the attachment.
    pub url: String,
    /// Optional human-readable name.
    pub name: Option<String>,
}

/// Type of attachment content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    /// Image file (PNG, JPG, etc.).
    Image,
    /// Generic file.
    File,
    /// Audio file.
    Audio,
    /// Video file.
    Video,
}

impl From<ChannelMessage> for llm::Message {
    fn from(msg: ChannelMessage) -> Self {
        llm::Message::user(msg.content)
    }
}

/// A connection to a messaging platform.
///
/// Uses associated types for platform-specific events and configuration.
/// Methods use RPITIT for async without boxing.
pub trait Channel: Send + Sync {
    /// Platform-specific event type yielded by the connection stream.
    type Event: Send;
    /// Platform-specific configuration for connecting.
    type Config: Send;

    /// The platform this channel connects to.
    fn platform(&self) -> Platform;

    /// Open a connection and return a stream of events.
    fn connect(
        &self,
        config: Self::Config,
    ) -> impl Future<Output = anyhow::Result<impl Stream<Item = Self::Event> + Send>> + Send;

    /// Send a message through the channel.
    fn send(&self, message: ChannelMessage) -> impl Future<Output = anyhow::Result<()>> + Send;
}

#[cfg(test)]
mod tests {
    use crate::channel::{AttachmentKind, ChannelMessage, Platform};

    #[test]
    fn platform_enum_variants() {
        let p = Platform::Telegram;
        assert_eq!(p, Platform::Telegram);
    }

    #[test]
    fn channel_message_to_llm_message() {
        let msg = ChannelMessage {
            platform: Platform::Telegram,
            channel_id: "chat123".into(),
            sender_id: "user456".into(),
            content: "hello agent".into(),
            attachments: vec![],
            reply_to: None,
            timestamp: 1000,
        };
        let llm_msg: llm::Message = msg.into();
        assert_eq!(llm_msg.content, "hello agent");
        assert_eq!(llm_msg.role, llm::Role::User);
    }

    #[test]
    fn attachment_kind_variants() {
        let kinds = [
            AttachmentKind::Image,
            AttachmentKind::File,
            AttachmentKind::Audio,
            AttachmentKind::Video,
        ];
        assert_eq!(kinds.len(), 4);
        assert_ne!(AttachmentKind::Image, AttachmentKind::File);
    }
}
