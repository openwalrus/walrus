//! Chat interfaces

pub use llama::Llama3;
use std::{fmt::Display, str::FromStr};

mod llama;

/// A message in a chat.
#[derive(Debug, Clone)]
pub enum Message {
    /// The assistant message
    Assistant(String),
    /// The user message
    User(String),
    /// The system message
    System(String),
}

impl Message {
    /// Get the text of the message
    pub fn text(&self) -> &str {
        match self {
            Self::Assistant(content) => content,
            Self::User(content) => content,
            Self::System(content) => content,
        }
    }

    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::User(content.into())
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant(content.into())
    }

    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::System(content.into())
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Assistant(content) => write!(f, "assistant: {}", content),
            Self::User(content) => write!(f, "user: {}", content),
            Self::System(content) => write!(f, "system: {}", content),
        }
    }
}

impl From<&str> for Message {
    fn from(s: &str) -> Self {
        Self::User(s.to_string())
    }
}

impl FromStr for Message {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (role, content) = s
            .split_once(": ")
            .ok_or_else(|| anyhow::anyhow!("invalid message format"))?;
        Ok(match role.to_lowercase().trim().as_ref() {
            "assistant" => Self::Assistant(content.to_string()),
            "user" => Self::User(content.to_string()),
            "system" => Self::System(content.to_string()),
            _ => anyhow::bail!("invalid role: {role}"),
        })
    }
}

/// A formatter for chat messages
pub trait Formatter {
    /// The end of stream token
    const EOS_TOKEN: &str;

    /// Format the messages
    fn format(messages: &[Message]) -> anyhow::Result<String>;

    /// Format a single message
    fn complete(message: &[Message]) -> anyhow::Result<String>;
}
