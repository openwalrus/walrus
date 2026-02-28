//! No-op LLM provider and registry for testing.
//!
//! Implements [`LLM`] and [`Registry`] but panics on `send` and `stream`.
//! Intended for unit tests that exercise tool dispatch, memory, and session
//! logic without making real LLM calls.

use crate::model::{General, LLM, Message, Registry, Response, StreamChunk};
use anyhow::Result;
use compact_str::CompactString;
use futures_core::Stream;

/// A no-op LLM provider that panics on any actual LLM call.
///
/// # Panics
///
/// Both `send` and `stream` panic if called. Only use this provider
/// in tests that never invoke LLM methods.
#[derive(Clone, Copy)]
pub struct NoopProvider;

impl LLM for NoopProvider {
    type ChatConfig = General;

    async fn send(&self, _config: &General, _messages: &[Message]) -> Result<Response> {
        panic!("NoopProvider::send called — not intended for real LLM calls");
    }

    fn stream(
        &self,
        _config: General,
        _messages: &[Message],
        _usage: bool,
    ) -> impl Stream<Item = Result<StreamChunk>> {
        async_stream::stream! {
            panic!("NoopProvider::stream called — not intended for real LLM calls");
            #[allow(unreachable_code)]
            {
                yield Ok(StreamChunk::separator());
            }
        }
    }
}

impl Registry for NoopProvider {
    async fn send(
        &self,
        _model: &str,
        _config: &General,
        _messages: &[Message],
    ) -> Result<Response> {
        panic!("NoopProvider::send called — not intended for real LLM calls");
    }

    fn stream(
        &self,
        _model: &str,
        _config: General,
        _messages: &[Message],
        _usage: bool,
    ) -> Result<impl Stream<Item = Result<StreamChunk>> + Send> {
        panic!("NoopProvider::stream called — not intended for real LLM calls");
        #[allow(unreachable_code)]
        Ok(async_stream::stream! {
            yield Ok(StreamChunk::separator());
        })
    }

    fn context_limit(&self, _model: &str) -> usize {
        64_000
    }

    fn active_model(&self) -> CompactString {
        CompactString::from("noop")
    }
}
