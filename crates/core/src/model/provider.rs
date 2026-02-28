//! Provider abstractions for the unified LLM Interfaces

use crate::model::{Config, Message, Response, StreamChunk};
use anyhow::Result;
use futures_core::Stream;

/// A trait for LLM providers.
///
/// Constructors are inherent methods on each provider — never called
/// polymorphically (DD#57).
pub trait LLM: Sized + Clone {
    /// The chat configuration.
    type ChatConfig: Config + Send;

    /// Send a message to the LLM
    fn send(
        &self,
        config: &Self::ChatConfig,
        messages: &[Message],
    ) -> impl Future<Output = Result<Response>> + Send;

    /// Send a message to the LLM with streaming
    fn stream(
        &self,
        config: Self::ChatConfig,
        messages: &[Message],
        usage: bool,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send;
}
