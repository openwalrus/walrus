//! Provider enum for runtime dispatch across LLM backends.
//!
//! Follows the same enum dispatch pattern as `MemoryBackend` (DD#22).
//! Each variant wraps a concrete provider. `impl LLM` delegates to the
//! inner provider, converting `General` config to the variant's native
//! request format via `From<General>`.

use anyhow::Result;
use async_stream::try_stream;
use claude::Claude;
use deepseek::DeepSeek;
use futures_core::Stream;
use futures_util::StreamExt;
use llm::{Client, General, LLM, Message, Response, StreamChunk};
use openai::OpenAI;

/// Unified LLM provider enum.
///
/// The gateway constructs the appropriate variant based on `ProviderKind`
/// in `LlmConfig`. The runtime is monomorphized on `Provider`.
#[derive(Clone)]
pub enum Provider {
    /// DeepSeek API.
    DeepSeek(DeepSeek),
    /// OpenAI-compatible API (covers OpenAI, Grok, Qwen, Kimi, Ollama).
    OpenAI(OpenAI),
    /// Anthropic Messages API.
    Claude(Claude),
}

impl LLM for Provider {
    type ChatConfig = General;

    fn new(client: Client, key: &str) -> Result<Self> {
        Ok(Self::DeepSeek(DeepSeek::new(client, key)?))
    }

    async fn send(&self, config: &General, messages: &[Message]) -> Result<Response> {
        match self {
            Self::DeepSeek(p) => {
                let req = deepseek::Request::from(config.clone());
                p.send(&req, messages).await
            }
            Self::OpenAI(p) => {
                let req = openai::Request::from(config.clone());
                p.send(&req, messages).await
            }
            Self::Claude(p) => {
                let req = claude::Request::from(config.clone());
                p.send(&req, messages).await
            }
        }
    }

    fn stream(
        &self,
        config: General,
        messages: &[Message],
        usage: bool,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send {
        let messages = messages.to_vec();
        let this = self.clone();
        try_stream! {
            match this {
                Provider::DeepSeek(p) => {
                    let req = deepseek::Request::from(config);
                    let mut stream = std::pin::pin!(p.stream(req, &messages, usage));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
                Provider::OpenAI(p) => {
                    let req = openai::Request::from(config);
                    let mut stream = std::pin::pin!(p.stream(req, &messages, usage));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
                Provider::Claude(p) => {
                    let req = claude::Request::from(config);
                    let mut stream = std::pin::pin!(p.stream(req, &messages, usage));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
            }
        }
    }
}
