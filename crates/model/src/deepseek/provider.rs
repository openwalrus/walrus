//! Model trait implementation for DeepSeek.

use super::DeepSeek;
use anyhow::Result;
use async_stream::try_stream;
use compact_str::CompactString;
use futures_core::Stream;
use futures_util::StreamExt;
use reqwest::Method;
use wcore::model::{Model, Response, StreamChunk};

const ENDPOINT: &str = "https://api.deepseek.com/chat/completions";

impl Model for DeepSeek {
    async fn send(&self, request: &wcore::model::Request) -> Result<Response> {
        let body = crate::request::Request::from(request.clone());
        tracing::trace!("request: {}", serde_json::to_string(&body)?);
        let text = self
            .client
            .request(Method::POST, ENDPOINT)
            .headers(self.headers.clone())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        serde_json::from_str(&text).map_err(Into::into)
    }

    fn stream(
        &self,
        request: wcore::model::Request,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send {
        let usage = request.usage;
        let body = crate::request::Request::from(request).stream(usage);
        if let Ok(body) = serde_json::to_string(&body) {
            tracing::trace!("request: {}", body);
        }
        let request = self
            .client
            .request(Method::POST, ENDPOINT)
            .headers(self.headers.clone())
            .json(&body);

        try_stream! {
            let response = request.send().await?;
            let mut stream = response.bytes_stream();
            while let Some(Ok(bytes)) = stream.next().await {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                tracing::trace!("chunk: {}", text);
                for data in text.split("data: ").skip(1).filter(|s| !s.starts_with("[DONE]")) {
                    let trimmed = data.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<StreamChunk>(trimmed) {
                        Ok(chunk) => yield chunk,
                        Err(e) => tracing::warn!("Failed to parse chunk: {e}, data: {}", trimmed),
                    }
                }
            }
        }
    }

    fn active_model(&self) -> CompactString {
        CompactString::from("deepseek-chat")
    }
}
