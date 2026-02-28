//! Unified LLM interface types and traits.
//!
//! Provides the shared types used across all LLM providers:
//! `Message`, `Response`, `StreamChunk`, `Tool`, `General`, and the `LLM` trait.

use anyhow::Result;
use futures_core::Stream;
pub use limits::default_context_limit;
pub use message::{Message, MessageBuilder, Role, estimate_tokens};
pub use request::General;
pub use response::{
    Choice, CompletionMeta, CompletionTokensDetails, Delta, FinishReason, Response, Usage,
};
pub use stream::StreamChunk;
pub use tool::{FunctionCall, Tool, ToolCall, ToolChoice};

mod limits;
mod message;
mod request;
mod response;
mod stream;
mod tool;

/// A trait for LLM providers.
///
/// Constructors are inherent methods on each provider — never called
/// polymorphically (DD#57).
pub trait Model: Sized + Clone {
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

impl Registry for () {
    async fn send(
        &self,
        _model: &str,
        _config: &General,
        _messages: &[Message],
    ) -> Result<Response> {
        anyhow::bail!("not implemented")
    }

    fn stream(
        &self,
        _model: &str,
        _config: General,
        _messages: &[Message],
        _usage: bool,
    ) -> Result<impl Stream<Item = Result<StreamChunk>> + Send> {
        Ok(async_stream::stream! {
            yield Err(anyhow::anyhow!("not implemented"));
        })
    }

    fn context_limit(&self, _model: &str) -> usize {
        0
    }

    fn active_model(&self) -> compact_str::CompactString {
        compact_str::CompactString::new("")
    }
}
