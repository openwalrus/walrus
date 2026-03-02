//! Fluent builder for constructing an [`Agent`].
//!
//! Requires an `mpsc::Sender<AgentEvent>` at construction — event emission
//! is not optional.

use crate::agent::Agent;
use crate::agent::config::AgentConfig;
use crate::event::AgentEvent;
use tokio::sync::mpsc;

/// Fluent builder for [`Agent`].
///
/// The event sender is required at construction — call [`AgentBuilder::new`]
/// with an `mpsc::Sender<AgentEvent>`. Use [`AgentConfig`] builder methods
/// for field configuration, then pass it via [`AgentBuilder::config`].
pub struct AgentBuilder {
    config: AgentConfig,
    event_tx: mpsc::Sender<AgentEvent>,
}

impl AgentBuilder {
    /// Create a new builder with the required event sender.
    pub fn new(event_tx: mpsc::Sender<AgentEvent>) -> Self {
        Self {
            config: AgentConfig::default(),
            event_tx,
        }
    }

    /// Set the full config, replacing all fields.
    ///
    /// Typical usage: build an `AgentConfig` via its fluent methods,
    /// then pass it here before calling `build()`.
    pub fn config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    /// Build the [`Agent`].
    pub fn build(self) -> Agent {
        Agent {
            config: self.config,
            history: Vec::new(),
            event_tx: self.event_tx,
        }
    }
}
