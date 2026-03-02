//! Hook trait — type-level runtime configuration.
//!
//! Hook tells the Runtime which model provider and memory backend to use,
//! and provides an event callback for observing agent execution.

use memory::{InMemory, Memory};
use wcore::AgentEvent;
use wcore::model::Model;

/// Type-level runtime configuration.
///
/// Determines the model provider and memory backend. Provides optional
/// event handling via `on_event()`.
pub trait Hook {
    /// The model provider for this hook.
    type Model: Model + Send + Sync;

    /// The memory backend for this hook.
    type Memory: Memory;

    /// Called when an agent emits an event during execution.
    ///
    /// Default implementation is a no-op. Override in daemon to forward
    /// events to connected clients.
    fn on_event(_event: &AgentEvent) {
        let _ = _event;
    }
}

impl Hook for () {
    type Model = ();
    type Memory = InMemory;
}
