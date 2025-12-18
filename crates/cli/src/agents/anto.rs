//! Anto agent - a basic agent to verify tool calling

use anyhow::Result;
use schemars::JsonSchema;
use ullm::{Agent, Message, StreamChunk, Tool, ToolCall, ToolMessage};

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
            description: "Gets the current date and time.".into(),
            parameters: schemars::schema_for!(GetTimeParams).into(),
            strict: true,
        }]
    }

    fn dispatch(&self, tools: &[ToolCall]) -> impl Future<Output = Vec<ToolMessage>> {
        async move {
            tools
                .iter()
                .map(|call| {
                    let result = match call.function.name.as_str() {
                        "get_time" => {
                            let now = std::time::SystemTime::now();
                            let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap();
                            let secs = duration.as_secs();
                            format!("Current Unix timestamp: {secs}")
                        }
                        _ => format!("Unknown tool: {}", call.function.name),
                    };
                    ToolMessage {
                        tool: call.id.clone(),
                        message: Message::tool(result),
                    }
                })
                .collect()
        }
    }

    async fn chunk(&self, chunk: &StreamChunk) -> Result<Self::Chunk> {
        Ok(chunk.clone())
    }
}
