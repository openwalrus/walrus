//! Provider implementation (DD#67).
//!
//! Unified `Provider` enum with enum dispatch over concrete backends.
//! `build_provider()` matches on `ProviderKind` detected from the model name.

use crate::claude::Claude;
use crate::config::{ProviderConfig, ProviderKind};
use crate::deepseek::DeepSeek;
use crate::openai::OpenAI;
use anyhow::Result;
use async_stream::try_stream;
use futures_core::Stream;
use futures_util::StreamExt;
use wcore::model::{General, Message, Model, Response, StreamChunk};

/// Unified LLM provider enum.
///
/// The gateway constructs the appropriate variant based on `ProviderKind`
/// detected from the model name. The runtime is monomorphized on `Provider`.
#[derive(Clone)]
pub enum Provider {
    /// DeepSeek API.
    DeepSeek(DeepSeek),
    /// OpenAI-compatible API (covers OpenAI, Grok, Qwen, Kimi, Ollama).
    OpenAI(OpenAI),
    /// Anthropic Messages API.
    Claude(Claude),
    /// Local inference via mistralrs.
    #[cfg(feature = "local")]
    Local(crate::local::Local),
}

impl Provider {
    /// Query the context length for a given model ID.
    ///
    /// Local providers delegate to mistralrs; remote providers return None
    /// (callers fall back to the static map in `wcore::model::default_context_limit`).
    pub fn context_length(&self, model: &str) -> Option<usize> {
        match self {
            Self::DeepSeek(_) | Self::OpenAI(_) | Self::Claude(_) => None,
            #[cfg(feature = "local")]
            Self::Local(p) => p.context_length(model),
        }
    }
}

/// Construct a `Provider` from config and a shared HTTP client.
///
/// This function is async because local providers need to load model weights
/// asynchronously.
pub async fn build_provider(config: &ProviderConfig, client: reqwest::Client) -> Result<Provider> {
    let kind = config.kind()?;
    let api_key = config.api_key.as_deref().unwrap_or("");
    let base_url = config.base_url.as_deref();

    let provider = match kind {
        ProviderKind::DeepSeek => match base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, api_key, url)?),
            None => Provider::DeepSeek(DeepSeek::new(client, api_key)?),
        },
        ProviderKind::OpenAI => match base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, api_key, url)?),
            None => Provider::OpenAI(OpenAI::api(client, api_key)?),
        },
        ProviderKind::Claude => match base_url {
            Some(url) => Provider::Claude(Claude::custom(client, api_key, url)?),
            None => Provider::Claude(Claude::anthropic(client, api_key)?),
        },
        ProviderKind::Grok => match base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, api_key, url)?),
            None => Provider::OpenAI(OpenAI::grok(client, api_key)?),
        },
        ProviderKind::Qwen => match base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, api_key, url)?),
            None => Provider::OpenAI(OpenAI::qwen(client, api_key)?),
        },
        ProviderKind::Kimi => match base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, api_key, url)?),
            None => Provider::OpenAI(OpenAI::kimi(client, api_key)?),
        },
        #[cfg(feature = "local")]
        ProviderKind::Local => {
            use crate::config::Loader;
            let loader = config.loader.unwrap_or_default();
            let isq = config.quantization.map(|q| q.to_isq());
            let chat_template = config.chat_template.as_deref();
            let local = match loader {
                Loader::Text => {
                    crate::local::Local::from_text(&config.model, isq, chat_template).await?
                }
                Loader::Gguf => {
                    crate::local::Local::from_gguf(&config.model, chat_template).await?
                }
                Loader::Vision => {
                    crate::local::Local::from_vision(&config.model, isq, chat_template).await?
                }
                Loader::Lora | Loader::XLora | Loader::GgufLora | Loader::GgufXLora => {
                    anyhow::bail!(
                        "loader {:?} requires adapter configuration (not yet supported)",
                        loader
                    );
                }
            };
            Provider::Local(local)
        }
        #[cfg(not(feature = "local"))]
        ProviderKind::Local => {
            anyhow::bail!("local provider requires the 'local' feature");
        }
    };
    Ok(provider)
}

impl Model for Provider {
    type ChatConfig = General;

    async fn send(&self, config: &General, messages: &[Message]) -> Result<Response> {
        match self {
            Self::DeepSeek(p) => {
                let req = crate::deepseek::Request::from(config.clone());
                p.send(&req, messages).await
            }
            Self::OpenAI(p) => {
                let req = crate::openai::Request::from(config.clone());
                p.send(&req, messages).await
            }
            Self::Claude(p) => {
                let req = crate::claude::Request::from(config.clone());
                p.send(&req, messages).await
            }
            #[cfg(feature = "local")]
            Self::Local(p) => p.send(config, messages).await,
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
                    let req = crate::deepseek::Request::from(config);
                    let mut stream = std::pin::pin!(p.stream(req, &messages, usage));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
                Provider::OpenAI(p) => {
                    let req = crate::openai::Request::from(config);
                    let mut stream = std::pin::pin!(p.stream(req, &messages, usage));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
                Provider::Claude(p) => {
                    let req = crate::claude::Request::from(config);
                    let mut stream = std::pin::pin!(p.stream(req, &messages, usage));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
                #[cfg(feature = "local")]
                Provider::Local(p) => {
                    let mut stream = std::pin::pin!(p.stream(config, &messages, usage));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
            }
        }
    }
}
