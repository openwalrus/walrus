//! Runner trait abstracting direct and gateway execution modes.
//!
//! Uses RPITIT (DD#11) — no dyn dispatch. The CLI binary dispatches
//! statically to either DirectRunner or GatewayRunner based on the
//! `--gateway` flag.

use anyhow::Result;
use futures_core::Stream;
use std::future::Future;

pub mod direct;
pub mod gateway;

/// Unified interface for sending messages and streaming responses.
pub trait Runner {
    /// Send a one-shot message and return the response content.
    fn send(&mut self, agent: &str, content: &str) -> impl Future<Output = Result<String>> + Send;

    /// Stream a response, yielding content text chunks.
    fn stream<'a>(
        &'a mut self,
        agent: &'a str,
        content: &'a str,
    ) -> impl Stream<Item = Result<String>> + Send + 'a;
}
