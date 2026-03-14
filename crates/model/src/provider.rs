//! Provider implementation.
//!
//! Unified `Provider` enum with enum dispatch over concrete backends.
//! `build_provider()` uses a URL lookup table for OpenAI-compatible kinds,
//! eliminating repeated match arms for each variant.

use crate::{
    config::{ApiStandard, ProviderConfig},
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
/// The gateway constructs the appropriate variant based on `ApiStandard`
/// from the provider config. The runtime is monomorphized on `Provider`.
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

    /// Wait until the provider is ready.
    ///
    /// No-op for remote providers. For local providers, blocks until the
    /// model finishes loading.
    pub async fn wait_until_ready(&mut self) -> Result<()> {
        match self {
            Self::OpenAI(_) | Self::Claude(_) => Ok(()),
            #[cfg(feature = "local")]
            Self::Local(p) => p.wait_until_ready().await,
        }
    }
}

/// Construct a remote `Provider` from config and a shared HTTP client.
///
/// Uses `effective_standard()` to pick the API protocol (OpenAI or Anthropic).
/// Local models are not handled here — they use the built-in registry.
pub async fn build_provider(config: &ProviderConfig, client: reqwest::Client) -> Result<Provider> {
    let api_key = config.api_key.as_deref().unwrap_or("");
    let model = config.name.as_str();

    match config.effective_standard() {
        ApiStandard::Anthropic => {
            let url = config.base_url.as_deref().unwrap_or(claude::ENDPOINT);
            Ok(Provider::Claude(Claude::custom(
                client, api_key, url, model,
            )?))
        }
        ApiStandard::OpenAI => {
            let url = config
                .base_url
                .as_deref()
                .unwrap_or(openai::endpoint::OPENAI);
            let provider = if api_key.is_empty() {
                OpenAI::no_auth(client, url, model)
            } else {
                OpenAI::custom(client, api_key, url, model)?
            };
            Ok(Provider::OpenAI(provider))
        }
    }
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
