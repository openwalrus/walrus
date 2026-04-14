//! Memory hook — thin facade over `crabtalk-memory`. Per-tool files
//! (`recall.rs`, `remember.rs`, `forget.rs`, `prompt.rs`) own the
//! corresponding `Memory` methods and `MemoryHook` dispatch handlers.
//! The `Hook` impl lives here because it isn't a tool.

use anyhow::Result;
use memory::Memory as Store;
use runtime::Hook;
use std::{
    path::PathBuf,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use wcore::{
    MemoryConfig, ToolDispatch, ToolFuture,
    agent::AsTool,
    model::{HistoryEntry, Tool},
};

mod forget;
mod prompt;
mod recall;
mod remember;

use forget::Forget;
use prompt::Prompt;
use recall::Recall;
use remember::Remember;

/// Shared handle to the underlying memory store. Cloneable because the
/// runtime needs a reference of its own for writing archives during
/// compaction and reading them back on session resume.
pub type SharedStore = Arc<RwLock<Store>>;

pub(super) const MEMORY_PROMPT: &str = include_str!("../../../prompts/memory.md");
pub const DEFAULT_SOUL: &str = include_str!("../../../prompts/crab.md");

/// Reserved entry name for the always-injected curated overview — what
/// used to be `MEMORY.md`. Named `global` because per-agent prompts
/// (v2) will live as sibling entries keyed by agent id.
pub const GLOBAL_PROMPT_NAME: &str = "global";

/// Reserved names users can't create/delete through `remember`/`forget`
/// — their content is load-bearing for the agent's system prompt.
pub(super) fn is_reserved(name: &str) -> bool {
    name == GLOBAL_PROMPT_NAME
}

pub struct Memory {
    pub(super) inner: SharedStore,
    pub(super) recall_limit: usize,
}

impl Memory {
    /// Open (or create) the memory db at `db_path`.
    pub fn open(config: MemoryConfig, db_path: PathBuf) -> Result<Self> {
        let store = Store::open(&db_path)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            recall_limit: config.recall_limit,
        })
    }

    /// Clone the underlying store handle. Used to hand the same memory
    /// to the runtime for archive writes and resume-time reads.
    pub fn shared(&self) -> SharedStore {
        self.inner.clone()
    }

    /// `unwrap` is intentional: nothing in `Memory` panics while holding
    /// the store guard, so poisoning would only happen if a future caller
    /// breaks that contract — and at that point a panic is the right
    /// failure mode.
    pub(super) fn store_read(&self) -> RwLockReadGuard<'_, Store> {
        self.inner.read().unwrap()
    }

    pub(super) fn store_write(&self) -> RwLockWriteGuard<'_, Store> {
        self.inner.write().unwrap()
    }
}

pub struct MemoryHook {
    pub(super) memory: Arc<Memory>,
}

impl MemoryHook {
    pub fn new(memory: Arc<Memory>) -> Self {
        Self { memory }
    }
}

impl Hook for MemoryHook {
    fn schema(&self) -> Vec<Tool> {
        vec![
            Recall::as_tool(),
            Remember::as_tool(),
            Forget::as_tool(),
            Prompt::as_tool(),
        ]
    }

    fn system_prompt(&self) -> Option<String> {
        Some(self.memory.build_prompt())
    }

    fn on_before_run(
        &self,
        _agent: &str,
        _conversation_id: u64,
        history: &[HistoryEntry],
    ) -> Vec<HistoryEntry> {
        self.memory.before_run(history)
    }

    fn dispatch<'a>(&'a self, name: &'a str, call: ToolDispatch) -> Option<ToolFuture<'a>> {
        match name {
            "recall" => Some(Box::pin(self.handle_recall(call))),
            "remember" => Some(Box::pin(self.handle_remember(call))),
            "forget" => Some(Box::pin(self.handle_forget(call))),
            "memory" => Some(Box::pin(self.handle_memory(call))),
            _ => None,
        }
    }
}
