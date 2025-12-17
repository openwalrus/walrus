//! Turbofish LLM message

use serde::{Deserialize, Serialize};

/// A message in the chat
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    /// The content of the message
    pub content: String,

    /// The name of the message
    pub name: String,

    /// The role of the message
    pub role: Role,
}

impl Message {
    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            name: String::new(),
            content: content.into(),
        }
    }

    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            name: String::new(),
            content: content.into(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            name: String::new(),
            content: content.into(),
        }
    }

    /// Create a new tool message
    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            name: String::new(),
            content: content.into(),
        }
    }
}

/// A tool message in the chat
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolMessage {
    /// The message
    #[serde(flatten)]
    pub message: Message,

    /// The tool call id
    #[serde(alias = "tool_call_id")]
    pub tool: String,
}

/// An assistant message in the chat
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssistantMessage {
    /// The message
    #[serde(flatten)]
    pub message: Message,

    /// Whether to prefix the message
    pub prefix: bool,

    /// The reasoning content
    #[serde(alias = "reasoning_content")]
    pub reasoning: String,
}

/// The role of a message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Role {
    /// The user role
    #[serde(rename = "user")]
    User,
    /// The assistant role
    #[serde(rename = "assistant")]
    Assistant,
    /// The system role
    #[serde(rename = "system")]
    System,
    /// The tool role
    #[serde(rename = "tool")]
    Tool,
}
