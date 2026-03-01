//! Shared OpenAI-compatible request body (DD#58, DD#71).
//!
//! Superset of the fields used by DeepSeek, OpenAI, and Mistral. Fields
//! use `Option` + `skip_serializing_if` so provider-specific extras (like
//! DeepSeek's `thinking`) are simply absent when unused.

use serde::Serialize;
use serde_json::{Value, json};
use wcore::model::{Message, Tool, ToolChoice};

/// OpenAI-compatible chat completions request body.
#[derive(Debug, Clone, Serialize)]
pub struct Request {
    /// The messages to send.
    pub messages: Vec<Message>,
    /// The model identifier.
    pub model: String,
    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<Value>,
    /// Whether to return log probabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Value>,
    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<Value>,
    /// Response format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Value>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Stream options (e.g. include_usage).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<Value>,
    /// Whether to enable thinking (DeepSeek-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Value>,
    /// Temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<Value>,
    /// Tool choice control.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    /// Tools the model may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    /// Number of most likely tokens to return at each position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<Value>,
    /// Top-p sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<Value>,
}

impl Request {
    /// Enable streaming for the request.
    pub fn stream(mut self, usage: bool) -> Self {
        self.stream = Some(true);
        self.stream_options = if usage {
            Some(json!({ "include_usage": true }))
        } else {
            None
        };
        self
    }

    /// Set the tools for the request.
    fn with_tools(self, tools: Vec<Tool>) -> Self {
        let tools = tools
            .into_iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": json!(tool),
                })
            })
            .collect::<Vec<_>>();
        Self {
            tools: Some(json!(tools)),
            ..self
        }
    }

    /// Set the tool choice for the request.
    fn with_tool_choice(self, tool_choice: ToolChoice) -> Self {
        Self {
            tool_choice: match tool_choice {
                ToolChoice::None => Some(json!("none")),
                ToolChoice::Auto => Some(json!("auto")),
                ToolChoice::Required => Some(json!("required")),
                ToolChoice::Function(name) => Some(json!({
                    "type": "function",
                    "function": { "name": name }
                })),
            },
            ..self
        }
    }
}

impl From<wcore::model::Request> for Request {
    fn from(req: wcore::model::Request) -> Self {
        let mut wire = Self {
            messages: req.messages,
            model: req.model.to_string(),
            frequency_penalty: None,
            logprobs: None,
            max_tokens: None,
            presence_penalty: None,
            response_format: None,
            stop: None,
            stream: None,
            stream_options: None,
            thinking: if req.think {
                Some(json!({ "type": "enabled" }))
            } else {
                None
            },
            temperature: None,
            tool_choice: None,
            tools: None,
            top_logprobs: None,
            top_p: None,
        };

        if let Some(tools) = req.tools {
            wire = wire.with_tools(tools);
        }
        if let Some(tool_choice) = req.tool_choice {
            wire = wire.with_tool_choice(tool_choice);
        }

        wire
    }
}
