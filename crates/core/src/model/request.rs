//! Chat request type (DD#69).

use crate::model::{Message, Tool, ToolChoice};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// A chat completion request.
///
/// Contains everything needed to make an LLM call: model, messages, tools,
/// and streaming hints. Provider implementations convert this to their
/// wire format via `From<Request>`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Request {
    /// The model to use.
    pub model: CompactString,

    /// The conversation messages.
    #[serde(default)]
    pub messages: Vec<Message>,

    /// Whether to enable thinking.
    pub think: bool,

    /// The tools available for this request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Controls which tool is called by the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Whether to return usage information in stream mode.
    pub usage: bool,
}

impl Request {
    /// Create a new request for the given model.
    pub fn new(model: impl Into<CompactString>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            think: false,
            tools: None,
            tool_choice: None,
            usage: false,
        }
    }

    /// Set the messages for this request.
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    /// Set the tools for this request.
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the tool choice for this request.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }
}

impl Default for Request {
    fn default() -> Self {
        Self {
            model: "deepseek-chat".into(),
            messages: Vec::new(),
            think: false,
            tools: None,
            tool_choice: None,
            usage: false,
        }
    }
}
