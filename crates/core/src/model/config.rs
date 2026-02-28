//! Configuration for a chat

use crate::model::{Tool, ToolChoice};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// Chat configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct General {
    /// The model to use
    pub model: CompactString,

    /// Whether to enable thinking
    pub think: bool,

    /// The tools to use
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Controls which tool is called by the model
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Whether to return the usage information in stream mode
    pub usage: bool,

    /// Context window limit override (in tokens).
    /// If `None`, the provider uses its default for the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_limit: Option<usize>,
}

impl General {
    /// Create a new configuration
    pub fn new(model: impl Into<CompactString>) -> Self {
        Self {
            model: model.into(),
            think: false,
            tools: None,
            tool_choice: None,
            usage: false,
            context_limit: None,
        }
    }

    /// Set the tools for the request.
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the tool choice for the request.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }
}

impl Default for General {
    fn default() -> Self {
        Self {
            model: "deepseek-chat".into(),
            think: false,
            tools: None,
            tool_choice: None,
            usage: false,
            context_limit: None,
        }
    }
}
