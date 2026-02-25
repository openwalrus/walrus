//! Structured knowledge memory for LLM agents.
//!
//! Memory is **not chat history**. It is structured knowledge — extracted
//! facts, user preferences, agent persona — that gets compiled into the
//! system prompt.
//!
//! # Usage
//!
//! ```rust,ignore
//! use walrus_core::{Agent, InMemory, Memory, with_memory};
//!
//! let memory = InMemory::new();
//! memory.set("user", "Prefers short answers.");
//!
//! let agent = Agent::new("anto").system_prompt("You are helpful.");
//! let agent = with_memory(agent, &memory);
//! ```

use std::future::Future;
use std::sync::Mutex;
use compact_str::CompactString;
use serde_json::Value;
use crate::Agent;

/// A structured memory entry with metadata and optional embedding.
#[derive(Debug, Clone, Default)]
pub struct MemoryEntry {
    /// Entry key (identity string).
    pub key: CompactString,
    /// Entry value (unbounded content).
    pub value: String,
    /// Optional structured metadata (JSON).
    pub metadata: Option<Value>,
    /// Unix timestamp when the entry was created.
    pub created_at: u64,
    /// Unix timestamp when the entry was last accessed.
    pub accessed_at: u64,
    /// Number of times the entry has been accessed.
    pub access_count: u32,
    /// Optional embedding vector for semantic search.
    pub embedding: Option<Vec<f32>>,
}

/// Options controlling memory recall behavior.
#[derive(Debug, Clone, Default)]
pub struct RecallOptions {
    /// Maximum number of results (0 = implementation default).
    pub limit: usize,
    /// Filter by creation time range (start, end) in unix seconds.
    pub time_range: Option<(u64, u64)>,
    /// Minimum relevance score threshold (0.0–1.0).
    pub relevance_threshold: Option<f32>,
}

/// Structured knowledge memory for LLM agents.
///
/// Implementations store named key-value pairs that get compiled
/// into the system prompt via [`compile()`](Memory::compile).
///
/// Uses `&self` for all methods — implementations must handle
/// interior mutability (e.g. via `Mutex`).
pub trait Memory: Send + Sync {
    /// Get the value for a key (owned).
    fn get(&self, key: &str) -> Option<String>;

    /// Get all key-value pairs (owned).
    fn entries(&self) -> Vec<(String, String)>;

    /// Set (upsert) a key-value pair. Returns the previous value if the key existed.
    fn set(&self, key: impl Into<String>, value: impl Into<String>) -> Option<String>;

    /// Remove a key. Returns the removed value if it existed.
    fn remove(&self, key: &str) -> Option<String>;

    /// Compile all entries into a string for system prompt injection.
    fn compile(&self) -> String {
        let entries = self.entries();
        if entries.is_empty() {
            return String::new();
        }

        let mut out = String::from("<memory>\n");
        for (key, value) in &entries {
            out.push_str(&format!("<{key}>\n"));
            out.push_str(value);
            if !value.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&format!("</{key}>\n"));
        }
        out.push_str("</memory>");
        out
    }

    /// Store a key-value pair (async). Default delegates to `set`.
    fn store(
        &self,
        key: impl Into<String> + Send,
        value: impl Into<String> + Send,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        self.set(key, value);
        async { Ok(()) }
    }

    /// Search for relevant entries (async). Default returns empty.
    fn recall(
        &self,
        _query: &str,
        _options: RecallOptions,
    ) -> impl Future<Output = anyhow::Result<Vec<MemoryEntry>>> + Send {
        async { Ok(Vec::new()) }
    }

    /// Compile relevant entries for a query (async). Default delegates to `compile`.
    fn compile_relevant(&self, _query: &str) -> impl Future<Output = String> + Send {
        let compiled = self.compile();
        async move { compiled }
    }
}

/// In-memory store backed by `Mutex<Vec<(String, String)>>`.
#[derive(Default, Debug)]
pub struct InMemory {
    entries: Mutex<Vec<(String, String)>>,
}

impl InMemory {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a store pre-populated with entries.
    pub fn with_entries(entries: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            entries: Mutex::new(entries.into_iter().collect()),
        }
    }
}

impl Memory for InMemory {
    fn get(&self, key: &str) -> Option<String> {
        let entries = self.entries.lock().unwrap();
        entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    fn entries(&self) -> Vec<(String, String)> {
        self.entries.lock().unwrap().clone()
    }

    fn set(&self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        let key = key.into();
        let value = value.into();
        let mut entries = self.entries.lock().unwrap();
        if let Some(existing) = entries.iter_mut().find(|(k, _)| *k == key) {
            Some(std::mem::replace(&mut existing.1, value))
        } else {
            entries.push((key, value));
            None
        }
    }

    fn remove(&self, key: &str) -> Option<String> {
        let mut entries = self.entries.lock().unwrap();
        let idx = entries.iter().position(|(k, _)| k == key)?;
        Some(entries.remove(idx).1)
    }
}

/// Apply memory to an agent — appends compiled memory to the system prompt.
pub fn with_memory(mut agent: Agent, memory: &impl Memory) -> Agent {
    let compiled = memory.compile();
    if !compiled.is_empty() {
        agent.system_prompt = format!("{}\n\n{compiled}", agent.system_prompt);
    }
    agent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mem = InMemory::new();
        assert!(mem.get("user").is_none());

        mem.set("user", "likes rust");
        assert_eq!(mem.get("user").unwrap(), "likes rust");
    }

    #[test]
    fn upsert_returns_old() {
        let mem = InMemory::new();
        assert!(mem.set("user", "v1").is_none());

        let old = mem.set("user", "v2");
        assert_eq!(old.unwrap(), "v1");
        assert_eq!(mem.get("user").unwrap(), "v2");
    }

    #[test]
    fn remove_returns_value() {
        let mem = InMemory::with_entries([("a".into(), "1".into())]);
        let removed = mem.remove("a");
        assert_eq!(removed.unwrap(), "1");
        assert!(mem.entries().is_empty());
        assert!(mem.remove("a").is_none());
    }

    #[test]
    fn compile_empty() {
        let mem = InMemory::new();
        assert_eq!(mem.compile(), "");
    }

    #[test]
    fn compile_entries() {
        let mem = InMemory::new();
        mem.set("user", "Prefers short answers.");
        mem.set("persona", "You are cautious.");
        let compiled = mem.compile();
        assert_eq!(
            compiled,
            "<memory>\n\
             <user>\n\
             Prefers short answers.\n\
             </user>\n\
             <persona>\n\
             You are cautious.\n\
             </persona>\n\
             </memory>"
        );
    }

    #[test]
    fn with_memory_appends() {
        let mem = InMemory::new();
        mem.set("user", "Likes Rust.");
        let agent = Agent::new("test").system_prompt("You are helpful.");
        let agent = with_memory(agent, &mem);
        assert!(agent.system_prompt.starts_with("You are helpful."));
        assert!(agent.system_prompt.contains("<memory>"));
    }

    #[test]
    fn with_memory_empty_noop() {
        let mem = InMemory::new();
        let agent = Agent::new("test").system_prompt("You are helpful.");
        let agent = with_memory(agent, &mem);
        assert_eq!(agent.system_prompt, "You are helpful.");
    }

    #[tokio::test]
    async fn store_delegates_to_set() {
        let mem = InMemory::new();
        mem.store("key", "value").await.unwrap();
        assert_eq!(mem.get("key").unwrap(), "value");
    }

    #[tokio::test]
    async fn compile_relevant_delegates_to_compile() {
        let mem = InMemory::new();
        mem.set("user", "test");
        let relevant = mem.compile_relevant("anything").await;
        let compiled = mem.compile();
        assert_eq!(relevant, compiled);
    }

    #[test]
    fn memory_entry_default() {
        let entry = MemoryEntry::default();
        assert!(entry.key.is_empty());
        assert!(entry.value.is_empty());
        assert!(entry.metadata.is_none());
        assert_eq!(entry.created_at, 0);
        assert_eq!(entry.accessed_at, 0);
        assert_eq!(entry.access_count, 0);
        assert!(entry.embedding.is_none());
    }

    #[test]
    fn recall_options_default() {
        let opts = RecallOptions::default();
        assert_eq!(opts.limit, 0);
        assert!(opts.time_range.is_none());
        assert!(opts.relevance_threshold.is_none());
    }

    #[test]
    fn memory_entry_clone() {
        let entry = MemoryEntry {
            key: CompactString::new("user"),
            value: "likes rust".into(),
            metadata: Some(serde_json::json!({"source": "chat"})),
            created_at: 1000,
            accessed_at: 2000,
            access_count: 5,
            embedding: Some(vec![0.1, 0.2, 0.3]),
        };
        let cloned = entry.clone();
        assert_eq!(cloned.key, "user");
        assert_eq!(cloned.value, "likes rust");
        assert_eq!(cloned.metadata, entry.metadata);
        assert_eq!(cloned.created_at, 1000);
        assert_eq!(cloned.accessed_at, 2000);
        assert_eq!(cloned.access_count, 5);
        assert_eq!(cloned.embedding, Some(vec![0.1, 0.2, 0.3]));
    }
}
