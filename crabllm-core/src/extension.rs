use crate::{
    ApiError, BoxFuture, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Error,
    Prefix, storage_key,
};
use std::time::Instant;

/// Per-request metadata passed to extension hooks.
#[derive(Clone, Debug)]
pub struct RequestContext {
    pub request_id: String,
    pub model: String,
    pub provider: String,
    pub key_name: Option<String>,
    pub is_stream: bool,
    pub started_at: Instant,
}

/// Error returned by `Extension::on_request` to short-circuit the pipeline.
/// Converted to an HTTP response in the handler.
pub struct ExtensionError {
    pub status: u16,
    pub body: ApiError,
}

impl ExtensionError {
    pub fn new(status: u16, message: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            status,
            body: ApiError::new(message, kind),
        }
    }
}

/// Trait for request pipeline extensions (usage tracking, logging, rate limiting, etc.).
///
/// Extensions are registered at startup and receive hooks at each stage of request
/// processing. All methods have default no-op implementations except `name` and `prefix`.
///
/// Extensions must be `Send + Sync` for use across async handler tasks.
/// Hook methods return `BoxFuture` for dyn-compatibility.
pub trait Extension: Send + Sync {
    /// Human-readable name for this extension, used in logs and diagnostics.
    fn name(&self) -> &str;

    /// Fixed 4-byte prefix that namespaces this extension's storage keys.
    fn prefix(&self) -> Prefix;

    /// Build a full storage key by prepending this extension's prefix to `suffix`.
    fn storage_key(&self, suffix: &[u8]) -> Vec<u8> {
        storage_key(&self.prefix(), suffix)
    }

    /// Check for a cached response before provider dispatch. Return `Some` to
    /// skip the provider call entirely. Called for non-streaming requests only.
    fn on_cache_lookup(
        &self,
        _request: &ChatCompletionRequest,
    ) -> BoxFuture<'_, Option<ChatCompletionResponse>> {
        Box::pin(async { None })
    }

    /// Called post-auth, pre-dispatch. Return `Err` to short-circuit the pipeline
    /// (no provider call, no further extensions run).
    fn on_request(&self, _ctx: &RequestContext) -> BoxFuture<'_, Result<(), ExtensionError>> {
        Box::pin(async { Ok(()) })
    }

    /// Called after a non-streaming chat completion response arrives from the provider.
    fn on_response(
        &self,
        _ctx: &RequestContext,
        _request: &ChatCompletionRequest,
        _response: &ChatCompletionResponse,
    ) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }

    /// Called once per SSE chunk during a streaming response, before serialization.
    fn on_chunk(&self, _ctx: &RequestContext, _chunk: &ChatCompletionChunk) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }

    /// Called when the provider returns an error.
    fn on_error(&self, _ctx: &RequestContext, _error: &Error) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}
