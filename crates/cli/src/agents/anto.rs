//! Anto agent - a basic agent to verify tool calling

use anyhow::Result;
use chrono::Utc;
use schemars::JsonSchema;
use ullm::{Agent, Message, StreamChunk, Tool, ToolCall};

/// Anto - a basic agent with tools for testing tool calls
#[derive(Clone)]
pub struct Anto;

/// Parameters for the get_time tool
#[allow(dead_code)]
#[derive(JsonSchema)]
struct GetTimeParams {
    /// Optional timezone (e.g., "UTC", "Local"). Defaults to local time.
    timezone: Option<String>,
}

impl Agent for Anto {
    type Chunk = StreamChunk;

    const SYSTEM_PROMPT: &str = "You are Anto, a helpful assistant. You can get the current time.";

    fn tools() -> Vec<Tool> {
        vec![Tool {
            name: "get_time".into(),
            description: "Gets the current UTC time in ISO 8601 format.".into(),
            parameters: schemars::schema_for!(GetTimeParams).into(),
            strict: true,
        }]
    }

    fn dispatch(&self, tools: &[ToolCall]) -> impl Future<Output = Vec<Message>> {
        async move {
            tools
                .iter()
                .map(|call| {
                    let result = match call.function.name.as_str() {
                        "get_time" => {
                            Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
                        }
                        _ => format!("Unknown tool: {}", call.function.name),
                    };
                    Message::tool(result, call.id.clone())
                })
                .collect()
        }
    }

    async fn chunk(&self, chunk: &StreamChunk) -> Result<Self::Chunk> {
        Ok(chunk.clone())
    }
}
