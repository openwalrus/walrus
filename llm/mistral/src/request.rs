//! Request body for Mistral chat completions API.

use llm::{Config, General, Message, Tool, ToolChoice};
use serde::Serialize;
use serde_json::{Value, json};

/// The request body for Mistral chat completions API.
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
    /// Temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<Value>,
    /// Tool choice control.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    /// Tools the model may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    /// Top-p sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<Value>,
}

impl Request {
    /// Clone the request with the given messages.
    pub fn messages(&self, messages: &[Message]) -> Self {
        Self {
            messages: messages.to_vec(),
            ..self.clone()
        }
    }

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
}

impl From<General> for Request {
    fn from(config: General) -> Self {
        let mut req = Self {
            messages: Vec::new(),
            model: config.model.to_string(),
            frequency_penalty: None,
            logprobs: None,
            max_tokens: None,
            presence_penalty: None,
            response_format: None,
            stop: None,
            stream: None,
            stream_options: None,
            temperature: None,
            tool_choice: None,
            tools: None,
            top_p: None,
        };

        if let Some(tools) = config.tools {
            req = req.with_tools(tools);
        }
        if let Some(tool_choice) = config.tool_choice {
            req = req.with_tool_choice(tool_choice);
        }

        req
    }
}

impl Config for Request {
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

#[cfg(test)]
mod tests {
    use super::Request;
    use llm::{Config, General, Tool, ToolChoice};

    #[test]
    fn request_from_general_sets_model() {
        let general = General {
            model: "mistral-medium".into(),
            ..General::default()
        };
        let req = Request::from(general);
        assert_eq!(req.model, "mistral-medium");
    }

    #[test]
    fn request_from_general_tools() {
        let tool = Tool {
            name: "search".into(),
            description: "find docs".into(),
            parameters: schemars::schema_for!(String),
            strict: false,
        };
        let general = General {
            model: "mistral-small".into(),
            tools: Some(vec![tool]),
            ..General::default()
        };
        let req = Request::from(general);
        let tools = req.tools.expect("tools");
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "search");
    }

    #[test]
    fn request_from_general_tool_choice() {
        let general = General {
            model: "mistral-small".into(),
            tool_choice: Some(ToolChoice::Function("search".into())),
            ..General::default()
        };
        let req = Request::from(general);
        let choice = req.tool_choice.expect("tool_choice");
        assert_eq!(choice["type"], "function");
        assert_eq!(choice["function"]["name"], "search");
    }

    #[test]
    fn stream_sets_include_usage() {
        let req = Request::from(General::default()).stream(true);
        assert_eq!(req.stream, Some(true));
        let opts = req.stream_options.expect("stream options");
        assert_eq!(opts["include_usage"], true);
    }

    #[test]
    fn stream_without_usage_omits_stream_options() {
        let req = Request::from(General::default()).stream(false);
        assert_eq!(req.stream, Some(true));
        assert!(req.stream_options.is_none());
    }

    #[test]
    fn with_tool_choice_auto() {
        let req = Request::from(General::default()).with_tool_choice(ToolChoice::Auto);
        assert_eq!(
            req.tool_choice.expect("tool choice"),
            serde_json::json!("auto")
        );
    }

    #[test]
    fn with_tool_choice_none() {
        let req = Request::from(General::default()).with_tool_choice(ToolChoice::None);
        assert_eq!(
            req.tool_choice.expect("tool choice"),
            serde_json::json!("none")
        );
    }

    #[test]
    fn with_tool_choice_required() {
        let req = Request::from(General::default()).with_tool_choice(ToolChoice::Required);
        assert_eq!(
            req.tool_choice.expect("tool choice"),
            serde_json::json!("required")
        );
    }
}
