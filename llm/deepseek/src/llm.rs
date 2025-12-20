//! The LLM implementation

use crate::{DeepSeek, Request};
use anyhow::Result;
use async_stream::try_stream;
use ccore::{
    Client, LLM, Message, Response, StreamChunk,
    reqwest::{
        Method,
        header::{self, HeaderMap},
    },
};
use futures_core::Stream;
use futures_util::StreamExt;

const ENDPOINT: &str = "https://api.deepseek.com/chat/completions";

impl LLM for DeepSeek {
    /// The chat configuration.
    type ChatConfig = Request;

    /// Create a new LLM provider
    fn new(client: Client, key: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse()?);
        headers.insert(header::ACCEPT, "application/json".parse()?);
        headers.insert(header::AUTHORIZATION, format!("Bearer {}", key).parse()?);
        Ok(Self { client, headers })
    }

    /// Send a message to the LLM
    async fn send(&mut self, req: &Request, messages: &[Message]) -> Result<Response> {
        let body = req.messages(messages);
        tracing::debug!("request: {}", serde_json::to_string(&body)?);
        let text = self
            .client
            .request(Method::POST, ENDPOINT)
            .headers(self.headers.clone())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        tracing::debug!("response: {text}");
        serde_json::from_str(&text).map_err(Into::into)
        // self.client
        //     .request(Method::POST, ENDPOINT)
        //     .headers(self.headers.clone())
        //     .json(&Request::from(config).messages(messages))
        //     .send()
        //     .await?
        //     .json::<Response>()
        //     .await
        //     .map_err(Into::into)
    }

    /// Send a message to the LLM with streaming
    fn stream(
        &mut self,
        req: Request,
        messages: &[Message],
        usage: bool,
    ) -> impl Stream<Item = Result<StreamChunk>> {
        let body = req.messages(messages).stream(usage);
        tracing::debug!(
            "request: {}",
            serde_json::to_string(&body).unwrap_or_default()
        );
        let request = self
            .client
            .request(Method::POST, ENDPOINT)
            .headers(self.headers.clone())
            .json(&body);

        try_stream! {
            tracing::debug!("Sending request to DeepSeek API");
            let response = request.send().await?;
            tracing::debug!("DeepSeek API responded with status: {}", response.status());
            let mut stream = response.bytes_stream();
            let mut chunk_count = 0;
            let mut last_chunk_time = std::time::Instant::now();
            while let Some(chunk_result) = stream.next().await {
                let elapsed_since_last = last_chunk_time.elapsed();
                last_chunk_time = std::time::Instant::now();

                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::error!(
                            "DeepSeek stream error after {} chunks ({}s since last chunk): {:?}",
                            chunk_count,
                            elapsed_since_last.as_secs_f32(),
                            e
                        );
                        Err(e)?
                    }
                };

                let text = String::from_utf8_lossy(&bytes).into_owned();
                tracing::trace!("Raw SSE chunk ({} bytes, {}ms since last): {:?}",
                    bytes.len(),
                    elapsed_since_last.as_millis(),
                    &text[..text.len().min(200)]
                );

                for data in text.split("data: ").skip(1).filter(|s| !s.starts_with("[DONE]")) {
                    let trimmed = data.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<StreamChunk>(trimmed) {
                        Ok(chunk) => {
                            chunk_count += 1;
                            tracing::debug!("Parsed chunk #{}: content={:?}, reasoning={:?}, finish={:?}",
                                chunk_count,
                                chunk.content().map(|s| s.len()),
                                chunk.reasoning_content().map(|s| s.len()),
                                chunk.reason()
                            );
                            yield chunk;
                        }
                        Err(e) => tracing::warn!("Failed to parse chunk: {e}, data: {}", trimmed),
                    }
                }
                if text.contains("[DONE]") {
                    tracing::debug!("Received [DONE] marker after {} chunks", chunk_count);
                }
            }
            tracing::debug!("DeepSeek stream closed normally after {} chunks", chunk_count);
        }
    }
}
