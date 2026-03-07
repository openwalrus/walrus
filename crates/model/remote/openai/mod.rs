//! OpenAI-compatible LLM provider.
//!
//! Covers OpenAI, Grok (xAI), Qwen (Alibaba), Kimi (Moonshot), Ollama,
//! and any other service exposing the OpenAI chat completions API.

use compact_str::CompactString;
use reqwest::{Client, header::HeaderMap};

mod provider;
mod request;

/// OpenAI-compatible endpoint URLs.
pub mod endpoint {
    /// OpenAI chat completions.
    pub const OPENAI: &str = "https://api.openai.com/v1/chat/completions";
    /// DeepSeek chat completions.
    pub const DEEPSEEK: &str = "https://api.deepseek.com/chat/completions";
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
    /// The configured model name (used by `active_model()`).
    model: CompactString,
}

impl OpenAI {
    /// Create a provider targeting a custom OpenAI-compatible endpoint with Bearer auth.
    pub fn custom(client: Client, key: &str, endpoint: &str, model: &str) -> anyhow::Result<Self> {
        use reqwest::header;
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse()?);
        headers.insert(header::ACCEPT, "application/json".parse()?);
        headers.insert(header::AUTHORIZATION, format!("Bearer {key}").parse()?);
        Ok(Self {
            client,
            headers,
            endpoint: endpoint.to_owned(),
            model: CompactString::from(model),
        })
    }

    /// Create a provider targeting a custom endpoint without authentication (e.g. Ollama).
    pub fn no_auth(client: Client, endpoint: &str, model: &str) -> Self {
        use reqwest::header::{self, HeaderValue};
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        Self {
            client,
            headers,
            endpoint: endpoint.to_owned(),
            model: CompactString::from(model),
        }
    }
}
