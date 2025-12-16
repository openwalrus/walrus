//! Configuration for a chat

use crate::{Tool, ToolChoice};
use serde::{Deserialize, Serialize};

/// Chat configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// The frequency penalty of the model
    pub frequency: i8,

    /// Whether to response in JSON
    pub json: bool,

    /// Whether to return the log probabilities
    pub logprobs: bool,

    /// The model to use
    pub model: String,

    /// The presence penalty of the model
    pub presence: i8,

    /// Stop sequences to halt generation
    pub stop: Vec<String>,

    /// The temperature of the model
    pub temperature: f32,

    /// Whether to enable thinking
    pub think: bool,

    /// Controls which tool is called by the model
    pub tool_choice: ToolChoice,

    /// A list of tools the model may call
    pub tools: Vec<Tool>,

    /// The top probability of the model
    pub top_p: f32,

    /// The number of top log probabilities to return
    pub top_logprobs: usize,

    /// The number of max tokens to generate
    pub tokens: usize,

    /// Whether to return the usage information in stream mode
    pub usage: bool,
}

impl Config {
    /// Create a new configuration
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    /// Add a tool to the configuration
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Set tools for the configuration
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
        self
    }

    /// Set the tool choice for the configuration
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = choice;
        self
    }

    /// Set stop sequences for the configuration
    pub fn stop(mut self, sequences: Vec<String>) -> Self {
        self.stop = sequences;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            frequency: 0,
            json: false,
            logprobs: false,
            model: "deepseek-chat".into(),
            presence: 0,
            stop: Vec::new(),
            temperature: 1.0,
            think: false,
            tool_choice: ToolChoice::None,
            tools: Vec::new(),
            top_logprobs: 0,
            top_p: 1.0,
            tokens: 1000,
            usage: true,
        }
    }
}
