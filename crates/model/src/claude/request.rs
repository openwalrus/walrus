//! Request body for the Anthropic Messages API.

use serde::Serialize;
use serde_json::{Value, json};
use wcore::model::{Role, Tool, ToolChoice};

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
    /// Enable streaming for the request.
    pub fn stream(mut self) -> Self {
        self.stream = Some(true);
        self
    }

    /// Set the tools for the request.
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

    /// Set the tool choice for the request.
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

impl From<wcore::model::Request> for Request {
    fn from(req: wcore::model::Request) -> Self {
        let mut system = None;
        let mut anthropic_msgs = Vec::new();

        for msg in &req.messages {
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

        let mut result = Self {
            model: req.model.to_string(),
            max_tokens: 4096,
            system,
            messages: anthropic_msgs,
            stream: None,
            tools: None,
            tool_choice: None,
            temperature: None,
            top_p: None,
        };

        if let Some(tools) = req.tools {
            result = result.with_tools(tools);
        }
        if let Some(tool_choice) = req.tool_choice {
            result = result.with_tool_choice(tool_choice);
        }

        result
    }
}
