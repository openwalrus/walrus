//! Shared HTTP transport for OpenAI-compatible LLM providers (DD#58).
//!
//! `HttpProvider` wraps a `reqwest::Client` with pre-configured headers and
//! endpoint URL. Provides `send()` for non-streaming and `stream_sse()` for
//! Server-Sent Events streaming. Used by DeepSeek, OpenAI, and Mistral —
//! Claude uses its own transport (different SSE format).

use anyhow::Result;
use async_stream::try_stream;
use futures_core::Stream;
use futures_util::StreamExt;
use reqwest::{
    Client, Method,
    header::{self, HeaderMap, HeaderName, HeaderValue},
};
use serde::Serialize;
use wcore::model::{Response, StreamChunk};

/// Shared HTTP transport for OpenAI-compatible providers.
///
/// Holds a `reqwest::Client`, pre-built headers (auth + content-type),
/// and the target endpoint URL.
#[derive(Clone)]
pub struct HttpProvider {
    client: Client,
    headers: HeaderMap,
    endpoint: String,
}

impl HttpProvider {
    /// Create a provider with Bearer token authentication.
    pub fn bearer(client: Client, key: &str, endpoint: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(header::AUTHORIZATION, format!("Bearer {key}").parse()?);
        Ok(Self {
            client,
            headers,
            endpoint: endpoint.to_owned(),
        })
    }

    /// Create a provider without authentication (e.g. Ollama).
    pub fn no_auth(client: Client, endpoint: &str) -> Self {
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
        }
    }

    /// Create a provider with a custom header for authentication.
    ///
    /// Used by providers that don't use Bearer tokens (e.g. Anthropic
    /// uses `x-api-key`).
    pub fn custom_header(
        client: Client,
        header_name: &str,
        header_value: &str,
        endpoint: &str,
    ) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            header_name.parse::<HeaderName>()?,
            header_value.parse::<HeaderValue>()?,
        );
        Ok(Self {
            client,
            headers,
            endpoint: endpoint.to_owned(),
        })
    }

    /// Send a non-streaming request and deserialize the response as JSON.
    pub async fn send(&self, body: &impl Serialize) -> Result<Response> {
        tracing::trace!("request: {}", serde_json::to_string(body)?);
        let text = self
            .client
            .request(Method::POST, &self.endpoint)
            .headers(self.headers.clone())
            .json(body)
            .send()
            .await?
            .text()
            .await?;

        serde_json::from_str(&text).map_err(Into::into)
    }

    /// Stream an SSE response (OpenAI-compatible format).
    ///
    /// Parses `data: ` prefixed lines, skips `[DONE]` sentinel, and
    /// deserializes each chunk as [`StreamChunk`].
    pub fn stream_sse(
        &self,
        body: &impl Serialize,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send {
        if let Ok(body) = serde_json::to_string(body) {
            tracing::trace!("request: {}", body);
        }
        let request = self
            .client
            .request(Method::POST, &self.endpoint)
            .headers(self.headers.clone())
            .json(body);

        try_stream! {
            let response = request.send().await?;
            let mut stream = response.bytes_stream();
            while let Some(next) = stream.next().await {
                let bytes = next?;
                let text = String::from_utf8_lossy(&bytes);
                tracing::trace!("chunk: {}", text);
                for data in text.split("data: ").skip(1).filter(|s| !s.starts_with("[DONE]")) {
                    let trimmed = data.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<StreamChunk>(trimmed) {
                        Ok(chunk) => yield chunk,
                        Err(e) => tracing::warn!("failed to parse chunk: {e}, data: {trimmed}"),
                    }
                }
            }
        }
    }

    /// Get the endpoint URL.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Get a reference to the headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
}
