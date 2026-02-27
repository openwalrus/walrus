//! OpenAI-compatible LLM provider.
//!
//! Covers OpenAI, Grok (xAI), Qwen (Alibaba), Kimi (Moonshot), Ollama,
//! and any other service exposing the OpenAI chat completions API.

use llm::reqwest::{Client, header::HeaderMap};
pub use request::Request;

mod provider;
mod request;

/// OpenAI-compatible endpoint URLs.
pub mod endpoint {
    /// OpenAI chat completions.
    pub const OPENAI: &str = "https://api.openai.com/v1/chat/completions";
    /// Grok (xAI) chat completions.
    pub const GROK: &str = "https://api.x.ai/v1/chat/completions";
    /// Qwen (Alibaba DashScope) chat completions.
    pub const QWEN: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions";
    /// Kimi (Moonshot) chat completions.
    pub const KIMI: &str = "https://api.moonshot.cn/v1/chat/completions";
    /// Ollama local chat completions.
    pub const OLLAMA: &str = "http://localhost:11434/v1/chat/completions";
}

/// An OpenAI-compatible LLM provider.
#[derive(Clone)]
pub struct OpenAI {
    /// The HTTP client.
    pub client: Client,
    /// Request headers (authorization, content-type).
    headers: HeaderMap,
    /// Chat completions endpoint URL.
    endpoint: String,
}

impl OpenAI {
    /// Create a provider targeting the OpenAI API.
    pub fn api(client: Client, key: &str) -> anyhow::Result<Self> {
        Self::custom(client, key, endpoint::OPENAI)
    }

    /// Create a provider targeting the Grok (xAI) API.
    pub fn grok(client: Client, key: &str) -> anyhow::Result<Self> {
        Self::custom(client, key, endpoint::GROK)
    }

    /// Create a provider targeting the Qwen (DashScope) API.
    pub fn qwen(client: Client, key: &str) -> anyhow::Result<Self> {
        Self::custom(client, key, endpoint::QWEN)
    }

    /// Create a provider targeting the Kimi (Moonshot) API.
    pub fn kimi(client: Client, key: &str) -> anyhow::Result<Self> {
        Self::custom(client, key, endpoint::KIMI)
    }

    /// Create a provider targeting a local Ollama instance (no API key).
    pub fn ollama(client: Client) -> anyhow::Result<Self> {
        use llm::reqwest::header;
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse()?);
        headers.insert(header::ACCEPT, "application/json".parse()?);
        Ok(Self {
            client,
            headers,
            endpoint: endpoint::OLLAMA.to_owned(),
        })
    }

    /// Create a provider targeting a custom OpenAI-compatible endpoint.
    pub fn custom(client: Client, key: &str, endpoint: &str) -> anyhow::Result<Self> {
        use llm::reqwest::header;
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse()?);
        headers.insert(header::ACCEPT, "application/json".parse()?);
        headers.insert(header::AUTHORIZATION, format!("Bearer {key}").parse()?);
        Ok(Self {
            client,
            headers,
            endpoint: endpoint.to_owned(),
        })
    }
}
