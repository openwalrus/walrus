//! The request body for the DeepSeek API

use serde::Serialize;
use serde_json::{Number, Value, json};
use ucore::{ChatMessage, Config, Tool};

/// The request body for the DeepSeek API
#[derive(Debug, Clone, Serialize)]
pub struct Request {
    /// The frequency penalty to use for the response
    #[serde(skip_serializing_if = "Value::is_null")]
    pub frequency_penalty: Value,

    /// Whether to return the log probabilities
    #[serde(skip_serializing_if = "Value::is_null")]
    pub logprobs: Value,

    /// The maximum number of tokens to generate
    pub max_tokens: usize,

    /// The messages to send to the API
    pub messages: Vec<ChatMessage>,

    /// The model we are using
    pub model: String,

    /// The presence penalty to use for the response
    #[serde(skip_serializing_if = "Value::is_null")]
    pub presence_penalty: Value,

    /// The response format to use
    #[serde(skip_serializing_if = "Value::is_null")]
    pub response_format: Value,

    /// Stop sequences
    #[serde(skip_serializing_if = "Value::is_null")]
    pub stop: Value,

    /// Whether to stream the response
    pub stream: bool,

    /// Stream options
    #[serde(skip_serializing_if = "Value::is_null")]
    pub stream_options: Value,

    /// Whether to enable thinking
    #[serde(skip_serializing_if = "Value::is_null")]
    pub thinking: Value,

    /// The temperature to use for the response
    #[serde(skip_serializing_if = "Value::is_null")]
    pub temperature: Value,

    /// Controls which (if any) tool is called by the model
    #[serde(skip_serializing_if = "Value::is_null")]
    pub tool_choice: Value,

    /// A list of tools the model may call
    #[serde(skip_serializing_if = "Value::is_null")]
    pub tools: Value,

    /// An integer between 0 and 20 specifying the number of most likely tokens to
    /// return at each token position, each with an associated log probability.
    #[serde(skip_serializing_if = "Value::is_null")]
    pub top_logprobs: Value,

    /// The top probability to use for the response
    #[serde(skip_serializing_if = "Value::is_null")]
    pub top_p: Value,
}

impl Request {
    /// Construct the messages for the request
    pub fn messages(&mut self, messages: &[ChatMessage]) -> Self {
        Self {
            messages: messages.to_vec(),
            ..self.clone()
        }
    }

    /// Enable streaming for the request
    pub fn stream(mut self, usage: bool) -> Self {
        self.stream = true;
        self.stream_options = if usage {
            json!({ "include_usage": true })
        } else {
            Value::Null
        };
        self
    }
}

impl From<&Config> for Request {
    fn from(config: &Config) -> Self {
        Self {
            frequency_penalty: Number::from_f64(config.frequency as f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            logprobs: if config.logprobs {
                Value::Bool(true)
            } else {
                Value::Null
            },
            max_tokens: config.tokens,
            messages: Vec::new(),
            model: config.model.clone(),
            presence_penalty: Number::from_f64(config.presence as f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            response_format: if config.json {
                json!({ "type": "json_object" })
            } else {
                Value::Null
            },
            stop: if config.stop.is_empty() {
                Value::Null
            } else {
                config.stop.iter().map(|s| json!(s)).collect()
            },
            stream: false,
            stream_options: Value::Null,
            temperature: Number::from_f64(config.temperature as f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            thinking: if config.think {
                json!({ "type": "enabled" })
            } else {
                Value::Null
            },
            tool_choice: serde_json::to_value(&config.tool_choice).unwrap_or(Value::Null),
            tools: serialize_tools(&config.tools),
            top_logprobs: if config.logprobs {
                Value::Number(config.top_logprobs.into())
            } else {
                Value::Null
            },
            top_p: Number::from_f64(config.top_p as f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        }
    }
}

/// Serialize tools to JSON value
fn serialize_tools(tools: &[Tool]) -> Value {
    if tools.is_empty() {
        return Value::Null;
    }

    let tools: Vec<Value> = tools
        .iter()
        .map(|tool| json!({ "type": "function", "function": tool }))
        .collect();

    Value::Array(tools)
}
