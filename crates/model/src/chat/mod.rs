//! Chat interfaces

use std::{fmt::Display, str::FromStr};

/// A message in a chat.
#[derive(Debug, Clone, Default)]
pub struct Message {
    /// The role of the message.
    pub role: Role,
    /// The content of the message.
    ///
    /// NOTE: only supports string atm
    pub content: String,
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.role, self.content)
    }
}

impl From<&str> for Message {
    fn from(s: &str) -> Self {
        Self {
            role: Role::User,
            content: s.to_string(),
        }
    }
}

impl FromStr for Message {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (role, content) = s
            .split_once(": ")
            .ok_or_else(|| anyhow::anyhow!("invalid message format"))?;
        Ok(Self {
            role: Role::from_str(role)?,
            content: content.to_string(),
        })
    }
}

/// The role of the message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Role {
    /// The assistant.
    ///
    /// The content is the assistant's message.
    Assistant,
    /// The user.
    ///
    /// The content is the user's message.
    #[default]
    User,
    /// The system.
    ///
    /// The content is the system's message.
    System,
}

impl FromStr for Role {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "assistant" => Self::Assistant,
            "user" => Self::User,
            "system" => Self::System,
            _ => anyhow::bail!("invalid role: {s}"),
        })
    }
}
