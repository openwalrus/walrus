//! Provider implementation.
//!
//! Unified `Provider` enum with enum dispatch over concrete backends.
//! `build_provider()` uses a URL lookup table for OpenAI-compatible kinds,
//! eliminating repeated match arms for each variant.

use crate::{
    config::{ProviderConfig, ProviderKind},
    remote::{
        claude::{self, Claude},
        openai::{self, OpenAI},
    },
};
use anyhow::Result;
use async_stream::try_stream;
use compact_str::CompactString;
use futures_core::Stream;
use futures_util::StreamExt;
use wcore::model::{Model, Response, StreamChunk};

/// Unified LLM provider enum.
///
/// The gateway constructs the appropriate variant based on `ProviderKind`
/// detected from the model name. The runtime is monomorphized on `Provider`.
#[derive(Clone)]
pub enum Provider {
    /// OpenAI-compatible API (covers OpenAI, DeepSeek, Grok, Qwen, Kimi, Ollama).
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
    pub fn context_length(&self, _model: &str) -> Option<usize> {
        match self {
            Self::OpenAI(_) | Self::Claude(_) => None,
            #[cfg(feature = "local")]
            Self::Local(p) => p.context_length(_model),
        }
    }
}

/// Construct a `Provider` from config and a shared HTTP client.
///
/// OpenAI-compatible kinds use a URL lookup table — no repeated arms.
/// The `model` string from config is stored in the provider for accurate
/// `active_model()` reporting.
pub async fn build_provider(config: &ProviderConfig, client: reqwest::Client) -> Result<Provider> {
    let kind = config.kind()?;
    let api_key = config.api_key.as_deref().unwrap_or("");
    let model = config.model.as_str();

    match kind {
        ProviderKind::Claude => {
            let url = config.base_url.as_deref().unwrap_or(claude::ENDPOINT);
            return Ok(Provider::Claude(Claude::custom(
                client, api_key, url, model,
            )?));
        }
        #[cfg(feature = "local")]
        ProviderKind::Local => {
            use crate::config::Loader;

            // Auto-switch HF registry so mistralrs uses the fastest endpoint.
            let endpoint = crate::local::download::probe_endpoint().await;
            tracing::info!("using hf endpoint: {endpoint}");
            unsafe { std::env::set_var("HF_ENDPOINT", &endpoint) };

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
            return Ok(Provider::Local(local));
        }
        #[cfg(not(feature = "local"))]
        ProviderKind::Local => {
            anyhow::bail!("local provider requires the 'local' feature");
        }
        _ => {}
    }

    // All remaining kinds are OpenAI-compatible. Look up the default endpoint URL.
    let default_url: &str = match kind {
        ProviderKind::OpenAI => openai::endpoint::OPENAI,
        ProviderKind::DeepSeek => openai::endpoint::DEEPSEEK,
        ProviderKind::Grok => openai::endpoint::GROK,
        ProviderKind::Qwen => openai::endpoint::QWEN,
        ProviderKind::Kimi => openai::endpoint::KIMI,
        // Claude and Local are handled above; this arm is unreachable.
        _ => unreachable!(),
    };
    let url = config.base_url.as_deref().unwrap_or(default_url);
    let provider = if api_key.is_empty() {
        OpenAI::no_auth(client, url, model)
    } else {
        OpenAI::custom(client, api_key, url, model)?
    };
    Ok(Provider::OpenAI(provider))
}

impl Model for Provider {
    async fn send(&self, request: &wcore::model::Request) -> Result<Response> {
        match self {
            Self::OpenAI(p) => p.send(request).await,
            Self::Claude(p) => p.send(request).await,
            #[cfg(feature = "local")]
            Self::Local(p) => p.send(request).await,
        }
    }

    fn stream(
        &self,
        request: wcore::model::Request,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send {
        let this = self.clone();
        try_stream! {
            match this {
                Provider::OpenAI(p) => {
                    let mut stream = std::pin::pin!(p.stream(request));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
                Provider::Claude(p) => {
                    let mut stream = std::pin::pin!(p.stream(request));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
                #[cfg(feature = "local")]
                Provider::Local(p) => {
                    let mut stream = std::pin::pin!(p.stream(request));
                    while let Some(chunk) = stream.next().await {
                        yield chunk?;
                    }
                }
            }
        }
    }

    fn context_limit(&self, model: &str) -> usize {
        self.context_length(model)
            .unwrap_or_else(|| wcore::model::default_context_limit(model))
    }

    fn active_model(&self) -> CompactString {
        match self {
            Self::OpenAI(p) => p.active_model(),
            Self::Claude(p) => p.active_model(),
            #[cfg(feature = "local")]
            Self::Local(p) => p.active_model(),
        }
    }
}
