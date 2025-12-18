//! Tool abstractions for the unified LLM Interfaces

use schemars::Schema;
use serde::{Deserialize, Serialize};

/// A tool for the LLM
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tool {
    /// The name of the tool
    pub name: String,

    /// The description of the tool
    pub description: String,

    /// The parameters of the tool
    pub parameters: Schema,

    /// Whether to strictly validate the parameters
    pub strict: bool,
}

/// A tool call made by the model
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolCall {
    /// The ID of the tool call
    pub id: String,

    /// The type of tool (currently only "function")
    #[serde(rename = "type")]
    pub call_type: String,

    /// The function to call
    pub function: FunctionCall,
}

/// A function call within a tool call
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionCall {
    /// The name of the function to call
    pub name: String,

    /// The arguments to pass to the function (JSON string)
    pub arguments: String,
}

/// Controls which tool is called by the model
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ToolChoice {
    /// Model will not call any tool
    #[serde(rename = "none")]
    None,

    /// Model can pick between generating a message or calling tools
    #[serde(rename = "auto")]
    Auto,

    /// Model must call one or more tools
    #[serde(rename = "required")]
    Required,

    /// Model must call the specified function
    Function {
        r#type: String,
        function: ToolChoiceFunction,
    },
}

/// A specific function to call
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolChoiceFunction {
    /// The name of the function to call
    pub name: String,
}

impl ToolChoice {
    /// Create a tool choice for a specific function
    pub fn function(name: impl Into<String>) -> Self {
        ToolChoice::Function {
            r#type: "function".into(),
            function: ToolChoiceFunction { name: name.into() },
        }
    }
}
