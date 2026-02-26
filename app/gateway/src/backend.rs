//! Memory backend enum for static dispatch over memory implementations.
//!
//! Wraps [`InMemory`] and [`SqliteMemory<NoEmbedder>`] with Memory trait
//! delegation, following the Provider enum pattern (DD#22).

use agent::{Embedder, InMemory, Memory, MemoryEntry, RecallOptions};
use anyhow::Result;
use sqlite::SqliteMemory;
use std::future::Future;

/// A no-op embedder that returns empty vectors.
///
/// Used with [`SqliteMemory`] when no embedding model is configured.
pub struct NoEmbedder;

impl Embedder for NoEmbedder {
    async fn embed(&self, _text: &str) -> Vec<f32> {
        Vec::new()
    }
}

/// Memory backend selected from gateway configuration.
///
/// Delegates all [`Memory`] trait methods to the inner variant.
pub enum MemoryBackend {
    /// Volatile in-memory store.
    InMemory(InMemory),
    /// SQLite-backed persistent store (no embedder).
    Sqlite(SqliteMemory<NoEmbedder>),
}

impl MemoryBackend {
    /// Create from config: in-memory variant.
    pub fn in_memory() -> Self {
        Self::InMemory(InMemory::new())
    }

    /// Create from config: sqlite variant at the given path.
    pub fn sqlite(path: &str) -> Result<Self> {
        Ok(Self::Sqlite(SqliteMemory::open(path)?))
    }
}

impl Memory for MemoryBackend {
    fn get(&self, key: &str) -> Option<String> {
        match self {
            Self::InMemory(m) => m.get(key),
            Self::Sqlite(m) => m.get(key),
        }
    }

    fn entries(&self) -> Vec<(String, String)> {
        match self {
            Self::InMemory(m) => m.entries(),
            Self::Sqlite(m) => m.entries(),
        }
    }

    fn set(&self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        match self {
            Self::InMemory(m) => m.set(key, value),
            Self::Sqlite(m) => m.set(key, value),
        }
    }

    fn remove(&self, key: &str) -> Option<String> {
        match self {
            Self::InMemory(m) => m.remove(key),
            Self::Sqlite(m) => m.remove(key),
        }
    }

    fn store(
        &self,
        key: impl Into<String> + Send,
        value: impl Into<String> + Send,
    ) -> impl Future<Output = Result<()>> + Send {
        // Must eagerly convert to avoid borrow issues across await.
        let key = key.into();
        let value = value.into();
        async move {
            match self {
                Self::InMemory(m) => m.store(key, value).await,
                Self::Sqlite(m) => m.store(key, value).await,
            }
        }
    }

    fn recall(
        &self,
        query: &str,
        options: RecallOptions,
    ) -> impl Future<Output = Result<Vec<MemoryEntry>>> + Send {
        let query = query.to_owned();
        async move {
            match self {
                Self::InMemory(m) => m.recall(&query, options).await,
                Self::Sqlite(m) => m.recall(&query, options).await,
            }
        }
    }

    fn compile_relevant(&self, query: &str) -> impl Future<Output = String> + Send {
        let query = query.to_owned();
        async move {
            match self {
                Self::InMemory(m) => m.compile_relevant(&query).await,
                Self::Sqlite(m) => m.compile_relevant(&query).await,
            }
        }
    }
}
