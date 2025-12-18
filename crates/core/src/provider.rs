//! Provider abstractions for the unified LLM Interfaces

use crate::{Chat, ChatMessage, Config, General, Response, StreamChunk};
use anyhow::Result;
use futures_core::Stream;
use reqwest::Client;

/// A trait for LLM providers
pub trait LLM: Sized + Clone {
    /// The chat configuration.
    type ChatConfig: Config;

    /// Create a new LLM provider
    fn new(client: Client, key: &str) -> Result<Self>
    where
        Self: Sized;

    /// Create a new chat
    fn chat(&self, config: General) -> Chat<Self, ()> {
        Chat::new(config, self.clone())
    }

    /// Send a message to the LLM
    fn send(
        &mut self,
        config: &Self::ChatConfig,
        messages: &[ChatMessage],
    ) -> impl Future<Output = Result<Response>>;

    /// Send a message to the LLM with streaming
    fn stream(
        &mut self,
        config: Self::ChatConfig,
        messages: &[ChatMessage],
        usage: bool,
    ) -> impl Stream<Item = Result<StreamChunk>>;
}
