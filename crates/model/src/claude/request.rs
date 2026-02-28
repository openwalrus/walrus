//! Request body for the Anthropic Messages API.

use wcore::model::{Config, General, Message, Role, Tool, ToolChoice};
use serde::Serialize;
use serde_json::{Value, json};

/// The request body for the Anthropic Messages API.
#[derive(Debug, Clone, Serialize)]
pub struct Request {
    /// The model identifier.
    pub model: String,
    /// Maximum tokens to generate.
    pub max_tokens: usize,
    /// System prompt (top-level, not in messages array).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The messages array (Anthropic content block format).
    pub messages: Vec<Value>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Tools the model may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    /// Tool choice control.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    /// Temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Top-p sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
}

impl Request {
    /// Build the request with the given messages, converting from walrus
    /// `Message` format to Anthropic content block format.
    pub fn messages(&self, messages: &[Message]) -> Self {
        let mut system = self.system.clone();
        let mut anthropic_msgs = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    system = Some(msg.content.clone());
                }
                Role::User => {
                    anthropic_msgs.push(json!({
                        "role": "user",
                        "content": msg.content,
                    }));
                }
                Role::Assistant => {
                    let mut content = Vec::new();
                    if !msg.content.is_empty() {
                        content.push(json!({
                            "type": "text",
                            "text": msg.content,
                        }));
                    }
                    for tc in &msg.tool_calls {
                        let input: Value =
                            serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
                        content.push(json!({
                            "type": "tool_use",
                            "id": tc.id,
                            "name": tc.function.name,
                            "input": input,
                        }));
                    }
                    if content.is_empty() {
                        content.push(json!({
                            "type": "text",
                            "text": "",
                        }));
                    }
                    anthropic_msgs.push(json!({
                        "role": "assistant",
                        "content": content,
                    }));
                }
                Role::Tool => {
                    anthropic_msgs.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": msg.tool_call_id,
                            "content": msg.content,
                        }],
                    }));
                }
            }
        }

        Self {
            system,
            messages: anthropic_msgs,
            ..self.clone()
        }
    }

    /// Enable streaming for the request.
    pub fn stream(mut self) -> Self {
        self.stream = Some(true);
        self
    }
}

impl From<General> for Request {
    fn from(config: General) -> Self {
        let mut req = Self {
            model: config.model.to_string(),
            max_tokens: config.context_limit.unwrap_or(4096),
            system: None,
            messages: Vec::new(),
            stream: None,
            tools: None,
            tool_choice: None,
            temperature: None,
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
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.parameters,
                })
            })
            .collect::<Vec<_>>();
        Self {
            tools: Some(tools),
            ..self
        }
    }

    fn with_tool_choice(self, tool_choice: ToolChoice) -> Self {
        Self {
            tool_choice: match tool_choice {
                ToolChoice::None => Some(json!({"type": "none"})),
                ToolChoice::Auto => Some(json!({"type": "auto"})),
                ToolChoice::Required => Some(json!({"type": "any"})),
                ToolChoice::Function(name) => Some(json!({
                    "type": "tool",
                    "name": name,
                })),
            },
            ..self
        }
    }
}
