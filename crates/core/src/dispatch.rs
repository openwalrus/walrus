//! Dispatcher trait for generic tool dispatch.
//!
//! The [`Dispatcher`] trait provides batch tool dispatch with RPITIT async.
//! Implementations handle parallelism internally (e.g. concurrent execution
//! via tokio). Agent calls `dispatch()` with all tool calls from a single
//! LLM response.

use crate::model::Tool;
use anyhow::Result;
use std::future::Future;

/// Generic tool dispatcher.
///
/// Passed as a method param to `Agent::step()`. Implementations wrap a tool
/// registry, MCP bridge, or any other tool backend. Uses RPITIT for async
/// without boxing — callers monomorphize over concrete dispatcher types.
pub trait Dispatcher: Send + Sync {
    /// Dispatch a batch of tool calls. Each entry is `(method, params)`.
    ///
    /// Returns one result per call in the same order. Implementations may
    /// execute calls concurrently.
    fn dispatch(&self, calls: &[(&str, &str)]) -> impl Future<Output = Vec<Result<String>>> + Send;

    /// Return the tool schemas this dispatcher can handle.
    ///
    /// Agent uses this to populate `Request.tools` before calling the model.
    fn tools(&self) -> Vec<Tool>;
}
