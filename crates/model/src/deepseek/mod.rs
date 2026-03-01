//! DeepSeek LLM provider

use reqwest::{Client, header::HeaderMap};

mod provider;

/// The DeepSeek LLM provider
#[derive(Clone)]
pub struct DeepSeek {
    /// The HTTP client
    pub client: Client,
    /// The request headers
    headers: HeaderMap,
}

impl DeepSeek {
    /// Create a new DeepSeek provider with the given client and API key.
    pub fn new(client: Client, key: &str) -> anyhow::Result<Self> {
        use reqwest::header;
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse()?);
        headers.insert(header::ACCEPT, "application/json".parse()?);
        headers.insert(header::AUTHORIZATION, format!("Bearer {key}").parse()?);
        Ok(Self { client, headers })
    }
}
