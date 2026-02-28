//! Provider implementation

use crate::config::ProviderConfig;
use anyhow::Result;
use async_stream::try_stream;
use claude::Claude;
use deepseek::DeepSeek;
use futures_core::Stream;
use futures_util::StreamExt;
use llm::{General, LLM, Message, Response, StreamChunk};
use mistral::Mistral;
use openai::OpenAI;
use serde::{Deserialize, Serialize};

/// Supported LLM provider kinds.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// DeepSeek API (default).
    #[default]
    DeepSeek,
    /// OpenAI API.
    OpenAI,
    /// Grok (xAI) API — OpenAI-compatible.
    Grok,
    /// Qwen (Alibaba DashScope) API — OpenAI-compatible.
    Qwen,
    /// Kimi (Moonshot) API — OpenAI-compatible.
    Kimi,
    /// Ollama local API — OpenAI-compatible, no key required.
    Ollama,
    /// Claude (Anthropic) Messages API.
    Claude,
    /// Mistral chat completions API.
    Mistral,
}

/// Unified LLM provider enum.
///
/// The gateway constructs the appropriate variant based on `ProviderKind`
/// in config. The runtime is monomorphized on `Provider`.
#[derive(Clone)]
pub enum Provider {
    /// DeepSeek API.
    DeepSeek(DeepSeek),
    /// OpenAI-compatible API (covers OpenAI, Grok, Qwen, Kimi, Ollama).
    OpenAI(OpenAI),
    /// Anthropic Messages API.
    Claude(Claude),
    /// Mistral chat completions API.
    Mistral(Mistral),
}

/// Construct a `Provider` from config and a shared HTTP client.
///
/// `ProviderKind::DeepSeek` with a custom `base_url` maps to the OpenAI
/// variant (OpenAI-compatible endpoint). Same for Grok, Qwen, Kimi, Ollama
/// with custom URLs.
pub fn build_provider(config: &ProviderConfig, client: llm::Client) -> Result<Provider> {
    let key = &config.api_key;
    let provider = match config.provider {
        ProviderKind::DeepSeek => match &config.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::DeepSeek(DeepSeek::new(client, key)?),
        },
        ProviderKind::OpenAI => match &config.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::OpenAI(OpenAI::api(client, key)?),
        },
        ProviderKind::Grok => match &config.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::OpenAI(OpenAI::grok(client, key)?),
        },
        ProviderKind::Qwen => match &config.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::OpenAI(OpenAI::qwen(client, key)?),
        },
        ProviderKind::Kimi => match &config.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::OpenAI(OpenAI::kimi(client, key)?),
        },
        ProviderKind::Ollama => match &config.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::OpenAI(OpenAI::ollama(client)?),
        },
        ProviderKind::Claude => match &config.base_url {
            Some(url) => Provider::Claude(Claude::custom(client, key, url)?),
            None => Provider::Claude(Claude::anthropic(client, key)?),
        },
        ProviderKind::Mistral => match &config.base_url {
            Some(url) => Provider::Mistral(Mistral::custom(client, key, url)?),
            None => Provider::Mistral(Mistral::api(client, key)?),
        },
    };
    Ok(provider)
}

impl LLM for Provider {
    type ChatConfig = General;

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
            Self::Mistral(p) => {
                let req = mistral::Request::from(config.clone());
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
                Provider::Mistral(p) => {
                    let req = mistral::Request::from(config);
                    let mut stream = std::pin::pin!(p.stream(req, &messages, usage));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
            }
        }
    }
}
