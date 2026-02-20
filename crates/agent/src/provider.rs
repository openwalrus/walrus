//! Provider enum for static dispatch over LLM implementations.

use anyhow::Result;
use deepseek::DeepSeek;
use futures_core::Stream;
use llm::{Client, Config, General, LLM, Message, Response, StreamChunk, Tool, ToolChoice};

/// Unified LLM provider (static dispatch, no dyn).
#[derive(Clone)]
pub enum Provider {
    /// DeepSeek provider.
    DeepSeek(DeepSeek),
}

impl Provider {
    /// Create a provider from a model name.
    pub fn new(model: &str, client: Client, key: &str) -> Result<Self> {
        match model {
            s if s.starts_with("deepseek") => Ok(Self::DeepSeek(DeepSeek::new(client, key)?)),
            _ => anyhow::bail!("unknown provider for model: {model}"),
        }
    }

    /// Send a non-streaming request.
    pub async fn send(
        &mut self,
        config: &General,
        tools: &[Tool],
        tool_choice: ToolChoice,
        messages: &[Message],
    ) -> Result<Response> {
        match self {
            Self::DeepSeek(p) => {
                let cfg = deepseek::Request::from(config.clone())
                    .with_tools(tools.to_vec())
                    .with_tool_choice(tool_choice);
                p.send(&cfg, messages).await
            }
        }
    }

    /// Send a streaming request.
    pub fn stream(
        &mut self,
        config: &General,
        tools: &[Tool],
        tool_choice: ToolChoice,
        messages: &[Message],
    ) -> impl Stream<Item = Result<StreamChunk>> {
        match self {
            Self::DeepSeek(p) => {
                let cfg = deepseek::Request::from(config.clone())
                    .with_tools(tools.to_vec())
                    .with_tool_choice(tool_choice);
                p.stream(cfg, messages, config.usage)
            }
        }
    }
}
