//! Provider implementation
//!
//! Unified `Provider` enum with enum dispatch over concrete backends.
//! `build_provider()` is async to support local model loading (DD#66).

use crate::config::{BackendConfig, ProviderConfig, RemoteConfig};
use anyhow::Result;
use async_stream::try_stream;
use claude::Claude;
use deepseek::DeepSeek;
use futures_core::Stream;
use futures_util::StreamExt;
use llm::{General, LLM, Message, Response, StreamChunk};
use openai::OpenAI;

/// Unified LLM provider enum.
///
/// The gateway constructs the appropriate variant based on `BackendConfig`
/// in config. The runtime is monomorphized on `Provider`.
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
    Local(local::Local),
}

/// Construct a `Provider` from config and a shared HTTP client.
///
/// This function is async because local providers need to load model weights
/// asynchronously.
pub async fn build_provider(config: &ProviderConfig, client: llm::Client) -> Result<Provider> {
    let provider = match &config.backend {
        BackendConfig::DeepSeek(rc) => build_remote_deepseek(rc, client)?,
        BackendConfig::OpenAI(rc) => build_remote_openai(rc, client)?,
        BackendConfig::Grok(rc) => build_remote_grok(rc, client)?,
        BackendConfig::Qwen(rc) => build_remote_qwen(rc, client)?,
        BackendConfig::Kimi(rc) => build_remote_kimi(rc, client)?,
        BackendConfig::Ollama(oc) => match &oc.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, "", url)?),
            None => Provider::OpenAI(OpenAI::ollama(client)?),
        },
        BackendConfig::Claude(rc) => match &rc.base_url {
            Some(url) => Provider::Claude(Claude::custom(client, &rc.api_key, url)?),
            None => Provider::Claude(Claude::anthropic(client, &rc.api_key)?),
        },
        #[cfg(feature = "local")]
        BackendConfig::Local(lc) => build_local(lc).await?,
    };
    Ok(provider)
}

fn build_remote_deepseek(rc: &RemoteConfig, client: llm::Client) -> Result<Provider> {
    match &rc.base_url {
        Some(url) => Ok(Provider::OpenAI(OpenAI::custom(client, &rc.api_key, url)?)),
        None => Ok(Provider::DeepSeek(DeepSeek::new(client, &rc.api_key)?)),
    }
}

fn build_remote_openai(rc: &RemoteConfig, client: llm::Client) -> Result<Provider> {
    match &rc.base_url {
        Some(url) => Ok(Provider::OpenAI(OpenAI::custom(client, &rc.api_key, url)?)),
        None => Ok(Provider::OpenAI(OpenAI::api(client, &rc.api_key)?)),
    }
}

fn build_remote_grok(rc: &RemoteConfig, client: llm::Client) -> Result<Provider> {
    match &rc.base_url {
        Some(url) => Ok(Provider::OpenAI(OpenAI::custom(client, &rc.api_key, url)?)),
        None => Ok(Provider::OpenAI(OpenAI::grok(client, &rc.api_key)?)),
    }
}

fn build_remote_qwen(rc: &RemoteConfig, client: llm::Client) -> Result<Provider> {
    match &rc.base_url {
        Some(url) => Ok(Provider::OpenAI(OpenAI::custom(client, &rc.api_key, url)?)),
        None => Ok(Provider::OpenAI(OpenAI::qwen(client, &rc.api_key)?)),
    }
}

fn build_remote_kimi(rc: &RemoteConfig, client: llm::Client) -> Result<Provider> {
    match &rc.base_url {
        Some(url) => Ok(Provider::OpenAI(OpenAI::custom(client, &rc.api_key, url)?)),
        None => Ok(Provider::OpenAI(OpenAI::kimi(client, &rc.api_key)?)),
    }
}

#[cfg(feature = "local")]
async fn build_local(lc: &crate::config::LocalConfig) -> Result<Provider> {
    use anyhow::bail;

    let provider = if let Some(model_id) = &lc.model_id {
        let isq = lc.quantization.map(|q| q.to_isq());
        local::Local::from_hf(model_id, isq).await?
    } else if let Some(model_path) = &lc.model_path {
        local::Local::from_gguf(
            model_path,
            lc.model_files.clone(),
            lc.chat_template.as_deref(),
        )
        .await?
    } else {
        bail!("local provider requires either model_id or model_path");
    };
    Ok(Provider::Local(provider))
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
