//! `Model<P>` — newtype wrapper around an `Arc<P>` provider.
//!
//! Exposes `send` / `stream` over wcore types, doing the
//! `wcore::Request` ↔ `crabllm_core::ChatCompletionRequest` conversion at
//! the call site. The wrapper is the only place wcore touches crabllm wire
//! types — agents and runtime see only the wcore-typed surface.
//!
//! Cloning is cheap: `Model<P>` is `Arc<P>` internally, so `clone()` is one
//! refcount bump regardless of `P`. This lets `Runtime` hold a single Model
//! and clone it into every Agent at build time without any `P: Clone` bound.

use crate::model::{Request, Response, StreamChunk, convert};
use anyhow::{Context, Result};
use async_stream::try_stream;
use crabllm_core::Provider;
use futures_core::Stream;
use futures_util::StreamExt;
use std::sync::Arc;

/// A wcore-typed view over a `crabllm_core::Provider`.
///
/// Holds an `Arc<P>` so cloning is structural and `P` itself does not need
/// to implement `Clone`. The `'static` bound on `P` flows from the
/// streaming path: `P::chat_completion_stream` returns a `BoxStream<'static>`
/// whose construction requires the implementor to be `'static`. Baking
/// the bound into the struct definition lets every downstream impl
/// (`Agent<P>`, `Runtime<P,H>`) carry the same constraint without
/// repeating it on every method.
pub struct Model<P: Provider + 'static> {
    inner: Arc<P>,
}

impl<P: Provider + 'static> Model<P> {
    /// Wrap a provider in a `Model`. The provider is moved into a new
    /// `Arc`; use [`Model::from_arc`] if you already have one.
    pub fn new(provider: P) -> Self {
        Self {
            inner: Arc::new(provider),
        }
    }

    /// Wrap an existing `Arc<P>` without re-allocating.
    pub fn from_arc(provider: Arc<P>) -> Self {
        Self { inner: provider }
    }

    /// Send a chat completion request, converting between wcore and
    /// crabllm-core types at the boundary.
    pub async fn send(&self, request: &Request) -> Result<Response> {
        let ct_req = convert::to_ct_request(request);
        let ct_resp = self
            .inner
            .chat_completion(&ct_req)
            .await
            .with_context(|| format!("model send failed for '{}'", request.model))?;
        Ok(convert::from_ct_response(ct_resp))
    }

    /// Stream a chat completion response. The returned stream owns its
    /// converted request and a clone of the inner Arc, so it is `'static`
    /// and can be spawned freely.
    pub fn stream(
        &self,
        request: Request,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send + 'static {
        let inner = Arc::clone(&self.inner);
        let ct_req = convert::to_ct_request(&request);
        let model_label = request.model.clone();
        try_stream! {
            let mut stream = inner
                .chat_completion_stream(&ct_req)
                .await
                .with_context(|| format!("model stream open failed for '{model_label}'"))?;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk
                    .with_context(|| format!("model stream chunk failed for '{model_label}'"))?;
                yield convert::from_ct_chunk(chunk);
            }
        }
    }
}

impl<P: Provider + 'static> Clone for Model<P> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<P: Provider + 'static> std::fmt::Debug for Model<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model").finish()
    }
}
