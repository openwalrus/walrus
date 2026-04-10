//! Hook trait — lifecycle backend for agent building, event observation,
//! tool schema registration, and persistence.
//!
//! All hook crates implement this trait. The runtime calls these
//! methods at the appropriate lifecycle points and reaches the
//! persistence backend through the [`Hook::Storage`] associated type.

use crate::{
    AgentConfig, AgentEvent, agent::tool::ToolRegistry, model::HistoryEntry, repos::Storage,
};
use std::{future::Future, sync::Arc};

/// Lifecycle backend for agent building, event observation, tool
/// registration, and persistence.
///
/// Implementors supply a concrete [`Storage`] type via the associated
/// item — the runtime reaches it through [`storage`](Self::storage)
/// and uses it for session persistence, memory entries, skill loading,
/// and agent storage. Non-persistence methods default to no-ops so
/// implementors only override what they need.
pub trait Hook: Send + Sync {
    /// Persistence backend this hook exposes to the runtime.
    type Storage: Storage;

    /// Shared handle to the persistence backend. Returns an Arc
    /// so callers can clone into spawned tasks when needed.
    fn storage(&self) -> &Arc<Self::Storage>;

    /// Called by `Runtime::add_agent()` before building the `Agent`.
    fn on_build_agent(&self, config: AgentConfig) -> AgentConfig {
        config
    }

    /// Called by Runtime after each agent step during execution.
    fn on_event(&self, _agent: &str, _conversation_id: u64, _event: &AgentEvent) {}

    /// Called by `Runtime::new()` to register tool schemas.
    fn on_register_tools(&self, _tools: &mut ToolRegistry) -> impl Future<Output = ()> + Send {
        async {}
    }

    /// Called by Runtime to preprocess user content before it becomes a message.
    fn preprocess(&self, _agent: &str, content: &str) -> String {
        content.to_owned()
    }

    /// Called by Runtime before each agent run (send_to / stream_to).
    fn on_before_run(
        &self,
        _agent: &str,
        _conversation_id: u64,
        _history: &[HistoryEntry],
    ) -> Vec<HistoryEntry> {
        Vec::new()
    }
}

/// Trivial [`Hook`] backed by [`InMemoryStorage`](crate::repos::mem::InMemoryStorage).
/// Useful in tests that need a `Runtime` but don't care about persistence.
#[cfg(feature = "test-utils")]
pub struct TestHook {
    storage: Arc<crate::repos::mem::InMemoryStorage>,
}

#[cfg(feature = "test-utils")]
impl Default for TestHook {
    fn default() -> Self {
        Self {
            storage: Arc::new(crate::repos::mem::InMemoryStorage::new()),
        }
    }
}

#[cfg(feature = "test-utils")]
impl TestHook {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(feature = "test-utils")]
impl Hook for TestHook {
    type Storage = crate::repos::mem::InMemoryStorage;

    fn storage(&self) -> &Arc<Self::Storage> {
        &self.storage
    }
}
