//! The LLM implementation

use crate::{DeepSeek, Request};
use anyhow::Result;
use async_stream::try_stream;
use futures_core::Stream;
use futures_util::StreamExt;
use ucore::{
    ChatMessage, Client, LLM, Response, StreamChunk,
    reqwest::{
        Method,
        header::{self, HeaderMap},
    },
};

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
    async fn send(&mut self, req: &Request, messages: &[ChatMessage]) -> Result<Response> {
        let text = self
            .client
            .request(Method::POST, ENDPOINT)
            .headers(self.headers.clone())
            .json(&req.messages(messages))
            .send()
            .await?
            .text()
            .await?;

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
        messages: &[ChatMessage],
        usage: bool,
    ) -> impl Stream<Item = Result<StreamChunk>> {
        let request = self
            .client
            .request(Method::POST, ENDPOINT)
            .headers(self.headers.clone())
            .json(&req.messages(messages).stream(usage));

        try_stream! {
            let mut stream = request.send().await?.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let text = String::from_utf8_lossy(&chunk?).into_owned();
                for data in text.split("data: ").skip(1).filter(|s| !s.starts_with("[DONE]")) {
                    yield serde_json::from_str(data.trim())?;
                }
            }
        }
    }
}
