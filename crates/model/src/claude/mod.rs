//! Claude (Anthropic) LLM provider.
//!
//! Implements the Anthropic Messages API, which differs from the OpenAI
//! chat completions format in message structure and streaming events.

use reqwest::{Client, header::HeaderMap};
pub use request::Request;

mod provider;
mod request;
mod stream;

/// The Anthropic Messages API endpoint.
pub const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

/// The Anthropic API version header value.
const API_VERSION: &str = "2023-06-01";

/// The Claude LLM provider.
#[derive(Clone)]
pub struct Claude {
    /// The HTTP client.
    pub client: Client,
    /// Request headers (x-api-key, anthropic-version, content-type).
    headers: HeaderMap,
    /// Messages API endpoint URL.
    endpoint: String,
}

impl Claude {
    /// Create a provider targeting the Anthropic API.
    pub fn anthropic(client: Client, key: &str) -> anyhow::Result<Self> {
        Self::custom(client, key, ENDPOINT)
    }

    /// Create a provider targeting a custom Anthropic-compatible endpoint.
    pub fn custom(client: Client, key: &str, endpoint: &str) -> anyhow::Result<Self> {
        use reqwest::header;
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse()?);
        headers.insert("x-api-key", key.parse()?);
        headers.insert("anthropic-version", API_VERSION.parse()?);
        Ok(Self {
            client,
            headers,
            endpoint: endpoint.to_owned(),
        })
    }
}
