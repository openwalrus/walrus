//! LLM trait implementation for the OpenAI-compatible provider.

use super::{OpenAI, Request};
use anyhow::Result;
use async_stream::try_stream;
use futures_core::Stream;
use futures_util::StreamExt;
use reqwest::Method;
use wcore::model::{Message, Model, Response, StreamChunk};

impl Model for OpenAI {
    type ChatConfig = Request;

    async fn send(&self, req: &Request, messages: &[Message]) -> Result<Response> {
        let body = req.messages(messages);
        tracing::trace!("request: {}", serde_json::to_string(&body)?);
        let text = self
            .client
            .request(Method::POST, &self.endpoint)
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
        req: Request,
        messages: &[Message],
        usage: bool,
    ) -> impl Stream<Item = Result<StreamChunk>> {
        let body = req.messages(messages).stream(usage);
        if let Ok(body) = serde_json::to_string(&body) {
            tracing::trace!("request: {}", body);
        }
        let request = self
            .client
            .request(Method::POST, &self.endpoint)
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
                        Err(e) => tracing::warn!("failed to parse chunk: {e}, data: {trimmed}"),
                    }
                }
            }
        }
    }
}
