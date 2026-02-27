//! Mistral LLM provider.
//!
//! Supports the Mistral chat completions API and compatible endpoints.

use llm::reqwest::{Client, header::HeaderMap};
pub use request::Request;

mod provider;
mod request;

/// Mistral endpoint URLs.
pub mod endpoint {
    /// Mistral chat completions endpoint.
    pub const MISTRAL: &str = "https://api.mistral.ai/v1/chat/completions";
}

/// Mistral provider.
#[derive(Clone)]
pub struct Mistral {
    /// The HTTP client.
    pub client: Client,
    /// Request headers (authorization, content-type).
    headers: HeaderMap,
    /// Chat completions endpoint URL.
    endpoint: String,
}

impl Mistral {
    /// Create a provider targeting the Mistral API.
    pub fn api(client: Client, key: &str) -> anyhow::Result<Self> {
        Self::custom(client, key, endpoint::MISTRAL)
    }

    /// Create a provider targeting a custom Mistral-compatible endpoint.
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

#[cfg(test)]
mod tests {
    use super::{Mistral, endpoint};

    #[test]
    fn custom_constructor_sets_endpoint() {
        let client = llm::Client::new();
        let custom = "http://localhost:9999/v1/chat/completions";
        let provider = Mistral::custom(client, "test-key", custom).expect("provider");
        assert_eq!(provider.endpoint, custom);
    }

    #[test]
    fn api_constructor_uses_default_endpoint() {
        let client = llm::Client::new();
        let provider = Mistral::api(client, "test-key").expect("provider");
        assert_eq!(provider.endpoint, endpoint::MISTRAL);
    }
}
