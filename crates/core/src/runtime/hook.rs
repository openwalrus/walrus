//! Hook trait — lifecycle backend for agent building, event observation,
//! tool schema registration, and persistence.
//!
//! All hook crates implement this trait. [`Runtime`](crate) calls these
//! methods at the appropriate lifecycle points and reaches the
//! persistence backend through the [`Hook::Storage`] associated type.

use crate::{AgentConfig, AgentEvent, Storage, agent::tool::ToolRegistry, model::HistoryEntry};
use std::{future::Future, sync::Arc};

/// Lifecycle backend for agent building, event observation, tool
/// registration, and persistence.
///
/// Implementors supply a concrete [`Storage`] type via the associated
/// [`Storage`](Self::Storage) item — the runtime reaches it through
/// [`storage`](Self::storage) and uses it for session persistence,
/// memory entries, and anything else that needs to outlive the
/// process. Non-storage methods default to no-ops so implementors only
/// override what they need.
pub trait Hook: Send + Sync {
    /// Persistence backend this hook exposes to the runtime.
    type Storage: Storage + 'static;

    /// Shared handle to the persistence backend. Conversation
    /// persistence, session replay, and subsystem state all route
    /// reads and writes through here.
    fn storage(&self) -> &Arc<Self::Storage>;

    /// Called by `Runtime::add_agent()` before building the `Agent`.
    ///
    /// Enriches the agent config: appends skill instructions, injects memory
    /// into the system prompt, etc. The returned config is passed to `AgentBuilder`.
    ///
    /// Default: returns config unchanged.
    fn on_build_agent(&self, config: AgentConfig) -> AgentConfig {
        config
    }

    /// Called by Runtime after each agent step during execution.
    ///
    /// Receives every `AgentEvent` produced during `send_to` and `stream_to`.
    /// Use for logging, metrics, persistence, or forwarding.
    ///
    /// Default: no-op.
    fn on_event(&self, _agent: &str, _conversation_id: u64, _event: &AgentEvent) {}

    /// Called by `Runtime::new()` to register tool schemas into the registry.
    ///
    /// Implementations call `tools.insert(tool)` with schema-only `Tool` values.
    /// No handlers or closures are stored — dispatch is handled by the daemon.
    ///
    /// Default: no-op async.
    fn on_register_tools(&self, _tools: &mut ToolRegistry) -> impl Future<Output = ()> + Send {
        async {}
    }

    /// Called by Runtime to preprocess user content before it becomes a message.
    ///
    /// Used to resolve slash commands (e.g. `/skill-name args` → skill body + args).
    /// Returns the transformed content string.
    ///
    /// Default: returns content unchanged.
    fn preprocess(&self, _agent: &str, content: &str) -> String {
        content.to_owned()
    }

    /// Called by Runtime before each agent run (send_to / stream_to).
    ///
    /// Receives the agent name, conversation ID, and conversation history
    /// (including the latest user message). Returns messages to inject
    /// before the user message for additional context (e.g. memory,
    /// per-session environment).
    ///
    /// Default: no injection.
    fn on_before_run(
        &self,
        _agent: &str,
        _conversation_id: u64,
        _history: &[HistoryEntry],
    ) -> Vec<HistoryEntry> {
        Vec::new()
    }
}

/// Trivial [`Hook`] with no lifecycle behaviour backed by an in-memory
/// [`MemStorage`](crate::MemStorage). Useful in tests that need a
/// `Runtime` but don't care about persistence beyond "don't crash".
#[cfg(feature = "test-utils")]
#[derive(Default)]
pub struct TestHook {
    storage: Arc<crate::MemStorage>,
}

#[cfg(feature = "test-utils")]
impl TestHook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a [`TestHook`] that shares an existing `MemStorage` — handy
    /// when two runtimes (e.g. pre- and post-reload) need to see the
    /// same session state.
    pub fn with_storage(storage: Arc<crate::MemStorage>) -> Self {
        Self { storage }
    }
}

#[cfg(feature = "test-utils")]
impl Hook for TestHook {
    type Storage = crate::MemStorage;

    fn storage(&self) -> &Arc<Self::Storage> {
        &self.storage
    }
}
