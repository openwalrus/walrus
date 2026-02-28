//! Provider abstractions for the unified LLM Interfaces

use crate::model::{General, Message, Response, StreamChunk};
use anyhow::Result;
use futures_core::Stream;

/// A trait for LLM providers.
///
/// Constructors are inherent methods on each provider — never called
/// polymorphically (DD#57).
pub trait LLM: Sized + Clone {
    /// The chat configuration.
    type ChatConfig: From<General> + Clone + Send;

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

/// A model registry that routes requests to named providers (DD#68).
///
/// Unlike [`LLM`] which represents a single provider, `Registry` manages
/// multiple providers and routes by model name. Used by the runtime for
/// per-agent model selection.
pub trait Registry: Clone {
    /// Send a request to the named model.
    fn send(
        &self,
        model: &str,
        config: &General,
        messages: &[Message],
    ) -> impl Future<Output = Result<Response>> + Send;

    /// Stream a response from the named model.
    fn stream(
        &self,
        model: &str,
        config: General,
        messages: &[Message],
        usage: bool,
    ) -> Result<impl Stream<Item = Result<StreamChunk>> + Send>;

    /// Resolve the context limit for a model.
    fn context_limit(&self, model: &str) -> usize;

    /// Get the active/default model name.
    fn active_model(&self) -> compact_str::CompactString;
}
