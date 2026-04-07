//! `TestProvider` — scripted implementation of `crabllm_core::Provider`
//! for use in unit tests and benchmarks.
//!
//! Each constructor takes a fixed sequence of responses or chunk batches
//! that the provider pops on every call. Mirrors the old `TestModel` shape
//! but speaks crabllm-core wire types so it plugs into `Model<P>` via the
//! real conversion path — tests exercise `convert.rs` instead of bypassing it.
//!
//! Errors out with `Error::Internal` when the script runs dry, which the
//! agent loop surfaces as an `AgentStopReason::Error` or a regular stream
//! error depending on which path was called.

use crabllm_core::{
    BoxStream, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Error, Provider,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

/// A mock provider that returns scripted responses in order.
///
/// Thread-safe via `Arc<Mutex<_>>` and `Clone` (cheap — clones share the
/// same underlying script). The provider trait requires `Send + Sync`, both
/// are satisfied.
#[derive(Clone, Default, Debug)]
pub struct TestProvider {
    responses: Arc<Mutex<VecDeque<ChatCompletionResponse>>>,
    chunks: Arc<Mutex<VecDeque<Vec<ChatCompletionChunk>>>>,
}

impl TestProvider {
    /// Create a new test provider with scripted `chat_completion` responses.
    pub fn new(responses: Vec<ChatCompletionResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into())),
            chunks: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Create a new test provider with scripted `chat_completion_stream`
    /// chunk batches. Each batch is yielded in full by a single stream call.
    pub fn with_chunks(chunks: Vec<Vec<ChatCompletionChunk>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::new())),
            chunks: Arc::new(Mutex::new(chunks.into())),
        }
    }

    /// Create a test provider with both chat_completion responses and
    /// chat_completion_stream chunk batches scripted.
    pub fn with_both(
        responses: Vec<ChatCompletionResponse>,
        chunks: Vec<Vec<ChatCompletionChunk>>,
    ) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into())),
            chunks: Arc::new(Mutex::new(chunks.into())),
        }
    }
}

impl Provider for TestProvider {
    async fn chat_completion(
        &self,
        _request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Error> {
        let mut responses = self.responses.lock().unwrap();
        responses.pop_front().ok_or_else(|| {
            Error::Internal(
                "TestProvider: no more scripted responses for chat_completion".into(),
            )
        })
    }

    async fn chat_completion_stream(
        &self,
        _request: &ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<ChatCompletionChunk, Error>>, Error> {
        let batch = {
            let mut all = self.chunks.lock().unwrap();
            all.pop_front()
        };
        match batch {
            Some(chunks) => {
                let stream = async_stream::stream! {
                    for chunk in chunks {
                        yield Ok(chunk);
                    }
                };
                Ok(Box::pin(stream))
            }
            None => Err(Error::Internal(
                "TestProvider: no more scripted chunks for chat_completion_stream".into(),
            )),
        }
    }
}
