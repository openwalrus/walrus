//! SQLite-backed memory for Walrus agents.
//!
//! Provides [`SqliteMemory`], a persistent [`Memory`](agent::Memory) implementation
//! using SQLite with FTS5 full-text search.
//!
//! All SQL lives in `sql/*.sql` files, loaded via `include_str!`.

use agent::{Embedder, Memory, MemoryEntry, RecallOptions};
use anyhow::Result;
use compact_str::CompactString;
use rusqlite::Connection;
use serde_json::Value;
use std::{future::Future, path::Path, sync::Mutex};

const SQL_SCHEMA: &str = include_str!("../sql/schema.sql");
const SQL_TOUCH_ACCESS: &str = include_str!("../sql/touch_access.sql");
const SQL_SELECT_VALUE: &str = include_str!("../sql/select_value.sql");
const SQL_SELECT_ENTRIES: &str = include_str!("../sql/select_entries.sql");
const SQL_UPSERT: &str = include_str!("../sql/upsert.sql");
const SQL_DELETE: &str = include_str!("../sql/delete.sql");
const SQL_UPSERT_FULL: &str = include_str!("../sql/upsert_full.sql");
const SQL_SELECT_ENTRY: &str = include_str!("../sql/select_entry.sql");
const SQL_RECALL_FTS: &str = include_str!("../sql/recall_fts.sql");

/// SQLite-backed memory store with optional embedding support.
///
/// Wraps a `rusqlite::Connection` in a `Mutex` for thread safety.
/// Generic over `E: Embedder` for optional vector search.
pub struct SqliteMemory<E: Embedder> {
    conn: Mutex<Connection>,
    embedder: Option<E>,
}

impl<E: Embedder> SqliteMemory<E> {
    /// Open or create a SQLite database at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let mem = Self {
            conn: Mutex::new(conn),
            embedder: None,
        };
        mem.init_schema()?;
        Ok(mem)
    }

    /// Create an in-memory database (useful for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let mem = Self {
            conn: Mutex::new(conn),
            embedder: None,
        };
        mem.init_schema()?;
        Ok(mem)
    }

    /// Attach an embedder for vector search.
    pub fn with_embedder(mut self, embedder: E) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Initialize the database schema.
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(SQL_SCHEMA)?;
        Ok(())
    }
}

impl<E: Embedder> Memory for SqliteMemory<E> {
    fn get(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        let now = now_unix();
        conn.execute(SQL_TOUCH_ACCESS, rusqlite::params![now as i64, key])
            .ok();
        conn.query_row(SQL_SELECT_VALUE, [key], |row| row.get(0))
            .ok()
    }

    fn entries(&self) -> Vec<(String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(SQL_SELECT_ENTRIES).unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    fn set(&self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        let key = key.into();
        let value = value.into();
        let conn = self.conn.lock().unwrap();
        let now = now_unix() as i64;

        let old: Option<String> = conn
            .query_row(SQL_SELECT_VALUE, [&key], |row| row.get(0))
            .ok();

        conn.execute(SQL_UPSERT, rusqlite::params![key, value, now])
            .ok();

        old
    }

    fn remove(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        let old: Option<String> = conn
            .query_row(SQL_SELECT_VALUE, [key], |row| row.get(0))
            .ok();
        if old.is_some() {
            conn.execute(SQL_DELETE, [key]).ok();
        }
        old
    }

    fn store(
        &self,
        key: impl Into<String> + Send,
        value: impl Into<String> + Send,
    ) -> impl Future<Output = Result<()>> + Send {
        let key = key.into();
        let value = value.into();

        let conn = self.conn.lock().unwrap();
        let now = now_unix() as i64;

        conn.execute(SQL_UPSERT, rusqlite::params![key, value, now])
            .ok();

        async { Ok(()) }
    }

    fn recall(
        &self,
        query: &str,
        options: RecallOptions,
    ) -> impl Future<Output = Result<Vec<MemoryEntry>>> + Send {
        let result = self.recall_sync(query, &options);
        async move { result }
    }

    fn compile_relevant(&self, query: &str) -> impl Future<Output = String> + Send {
        let opts = RecallOptions {
            limit: 5,
            ..Default::default()
        };
        let entries = self.recall_sync(query, &opts).unwrap_or_default();
        let compiled = if entries.is_empty() {
            String::new()
        } else {
            let mut out = String::from("<memory>\n");
            for entry in &entries {
                out.push_str(&format!("<{}>\n", entry.key));
                out.push_str(&entry.value);
                if !entry.value.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str(&format!("</{}>\n", entry.key));
            }
            out.push_str("</memory>");
            out
        };
        async move { compiled }
    }
}

impl<E: Embedder> SqliteMemory<E> {
    /// Execute the recall pipeline synchronously.
    ///
    /// 1. BM25 via FTS5 MATCH
    /// 2. Temporal decay (30-day half-life from accessed_at)
    /// 3. Optional relevance threshold filter
    /// 4. MMR re-ranking (Jaccard similarity, lambda 0.7)
    /// 5. Top-k truncation
    fn recall_sync(&self, query: &str, options: &RecallOptions) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let now = now_unix();
        let limit = if options.limit == 0 {
            10
        } else {
            options.limit
        };

        let mut stmt = conn.prepare(SQL_RECALL_FTS)?;

        let candidates: Vec<(MemoryEntry, f64)> = stmt
            .query_map([query], |row| {
                let key_str: String = row.get(0)?;
                let value: String = row.get(1)?;
                let meta_str: Option<String> = row.get(2)?;
                let created_at: i64 = row.get(3)?;
                let accessed_at: i64 = row.get(4)?;
                let access_count: i32 = row.get(5)?;
                let emb_blob: Option<Vec<u8>> = row.get(6)?;
                let rank: f64 = row.get(7)?;

                let metadata = meta_str.and_then(|s| serde_json::from_str(&s).ok());
                let embedding = emb_blob.map(|b| {
                    b.chunks_exact(4)
                        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        .collect()
                });

                let entry = MemoryEntry {
                    key: CompactString::new(key_str),
                    value,
                    metadata,
                    created_at: created_at as u64,
                    accessed_at: accessed_at as u64,
                    access_count: access_count as u32,
                    embedding,
                };
                Ok((entry, rank))
            })?
            .filter_map(|r| r.ok())
            .collect();

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        // Apply temporal decay: score * e^(-lambda * age_days).
        // Half-life = 30 days -> lambda = ln(2) / 30.
        let lambda = std::f64::consts::LN_2 / 30.0;
        let mut scored: Vec<(MemoryEntry, f64)> = candidates
            .into_iter()
            .map(|(entry, bm25_rank)| {
                // bm25() returns negative values (more negative = more relevant).
                // Negate to get positive relevance score.
                let bm25_score = -bm25_rank;
                let age_days = now.saturating_sub(entry.accessed_at) as f64 / 86400.0;
                let decay = (-lambda * age_days).exp();
                let score = bm25_score * decay;
                (entry, score)
            })
            .collect();

        // Apply time_range filter on created_at.
        if let Some((start, end)) = options.time_range {
            scored.retain(|(entry, _)| entry.created_at >= start && entry.created_at <= end);
        }

        // Apply relevance threshold filter.
        if let Some(threshold) = options.relevance_threshold {
            scored.retain(|(_, score)| *score >= threshold as f64);
        }

        if scored.is_empty() {
            return Ok(Vec::new());
        }

        // MMR re-ranking (lambda = 0.7, Jaccard similarity).
        let reranked = mmr_rerank(scored, limit, 0.7);
        Ok(reranked)
    }

    /// Store a key-value pair with optional metadata and embedding.
    pub fn store_with_metadata(
        &self,
        key: &str,
        value: &str,
        metadata: Option<&Value>,
        embedding: Option<&[f32]>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = now_unix() as i64;
        let meta_json = metadata.map(|m| serde_json::to_string(m).unwrap());
        let emb_blob: Option<Vec<u8>> =
            embedding.map(|e| e.iter().flat_map(|f| f.to_le_bytes()).collect());

        conn.execute(
            SQL_UPSERT_FULL,
            rusqlite::params![key, value, meta_json, now, emb_blob],
        )?;
        Ok(())
    }

    /// Get a full MemoryEntry for a key.
    pub fn get_entry(&self, key: &str) -> Option<MemoryEntry> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(SQL_SELECT_ENTRY, [key], |row| {
            let key_str: String = row.get(0)?;
            let value: String = row.get(1)?;
            let meta_str: Option<String> = row.get(2)?;
            let created_at: i64 = row.get(3)?;
            let accessed_at: i64 = row.get(4)?;
            let access_count: i32 = row.get(5)?;
            let emb_blob: Option<Vec<u8>> = row.get(6)?;

            let metadata = meta_str.and_then(|s| serde_json::from_str(&s).ok());
            let embedding = emb_blob.map(|b| {
                b.chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect()
            });

            Ok(MemoryEntry {
                key: CompactString::new(key_str),
                value,
                metadata,
                created_at: created_at as u64,
                accessed_at: accessed_at as u64,
                access_count: access_count as u32,
                embedding,
            })
        })
        .ok()
    }
}

/// Jaccard similarity between two strings (tokenized by whitespace).
fn jaccard_similarity(a: &str, b: &str) -> f64 {
    use std::collections::HashSet;
    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

/// Maximal Marginal Relevance re-ranking.
///
/// Selects items that balance relevance (score) against diversity
/// (dissimilarity to already-selected items). Lambda controls the
/// trade-off: 1.0 = pure relevance, 0.0 = pure diversity.
fn mmr_rerank(
    candidates: Vec<(MemoryEntry, f64)>,
    limit: usize,
    mmr_lambda: f64,
) -> Vec<MemoryEntry> {
    let mut remaining: Vec<(MemoryEntry, f64)> = candidates;
    let mut selected: Vec<MemoryEntry> = Vec::with_capacity(limit);

    while selected.len() < limit && !remaining.is_empty() {
        let mut best_idx = 0;
        let mut best_mmr = f64::NEG_INFINITY;

        for (i, (entry, score)) in remaining.iter().enumerate() {
            let max_sim = selected
                .iter()
                .map(|s| jaccard_similarity(&entry.value, &s.value))
                .fold(0.0_f64, f64::max);
            let mmr_score = mmr_lambda * score - (1.0 - mmr_lambda) * max_sim;
            if mmr_score > best_mmr {
                best_mmr = mmr_score;
                best_idx = i;
            }
        }

        let (entry, _) = remaining.remove(best_idx);
        selected.push(entry);
    }

    selected
}

/// Return the current unix timestamp in seconds.
fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use crate::SqliteMemory;
    use agent::Memory;

    /// Noop embedder for tests that don't need vector search.
    struct NoopEmbedder;

    impl agent::Embedder for NoopEmbedder {
        fn embed(&self, _text: &str) -> impl std::future::Future<Output = Vec<f32>> + Send {
            async { vec![] }
        }
    }

    fn mem() -> SqliteMemory<NoopEmbedder> {
        SqliteMemory::<NoopEmbedder>::in_memory().unwrap()
    }

    #[test]
    fn open_in_memory() {
        let m = SqliteMemory::<NoopEmbedder>::in_memory();
        assert!(m.is_ok());
    }

    #[test]
    fn schema_created() {
        let m = mem();
        let conn = m.conn.lock().unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='memories'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='memories_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        let trigger_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='trigger' AND name LIKE 'memories_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(trigger_count, 3);
    }

    #[test]
    fn sqlite_set_and_get() {
        let m = mem();
        assert!(m.get("user").is_none());
        m.set("user", "likes rust");
        assert_eq!(m.get("user").unwrap(), "likes rust");
    }

    #[test]
    fn sqlite_upsert_returns_old() {
        let m = mem();
        assert!(m.set("user", "v1").is_none());
        let old = m.set("user", "v2");
        assert_eq!(old.unwrap(), "v1");
        assert_eq!(m.get("user").unwrap(), "v2");
    }

    #[test]
    fn sqlite_remove() {
        let m = mem();
        m.set("a", "1");
        let removed = m.remove("a");
        assert_eq!(removed.unwrap(), "1");
        assert!(m.get("a").is_none());
        assert!(m.remove("a").is_none());
    }

    #[test]
    fn sqlite_entries() {
        let m = mem();
        m.set("b", "2");
        m.set("a", "1");
        let entries = m.entries();
        assert_eq!(entries.len(), 2);
        // Ordered by key.
        assert_eq!(entries[0].0, "a");
        assert_eq!(entries[1].0, "b");
    }

    #[test]
    fn sqlite_compile() {
        let m = mem();
        m.set("user", "Prefers short answers.");
        let compiled = m.compile();
        assert!(compiled.contains("<memory>"));
        assert!(compiled.contains("<user>"));
        assert!(compiled.contains("Prefers short answers."));
        assert!(compiled.contains("</user>"));
        assert!(compiled.contains("</memory>"));
    }

    #[test]
    fn sqlite_get_updates_access_count() {
        let m = mem();
        m.set("key", "value");
        m.get("key");
        m.get("key");
        m.get("key");
        let entry = m.get_entry("key").unwrap();
        assert_eq!(entry.access_count, 3);
    }

    #[test]
    fn sqlite_store_with_metadata() {
        let m = mem();
        let meta = serde_json::json!({"source": "chat", "priority": 1});
        m.store_with_metadata("user", "likes rust", Some(&meta), None)
            .unwrap();
        let entry = m.get_entry("user").unwrap();
        assert_eq!(entry.value, "likes rust");
        let stored_meta = entry.metadata.unwrap();
        assert_eq!(stored_meta["source"], "chat");
        assert_eq!(stored_meta["priority"], 1);
    }

    #[test]
    fn sqlite_store_with_embedding() {
        let m = mem();
        let embedding = vec![0.1f32, 0.2, 0.3, 0.4];
        m.store_with_metadata("vec", "test", None, Some(&embedding))
            .unwrap();
        let entry = m.get_entry("vec").unwrap();
        let stored_emb = entry.embedding.unwrap();
        assert_eq!(stored_emb.len(), 4);
        assert!((stored_emb[0] - 0.1).abs() < 1e-6);
        assert!((stored_emb[3] - 0.4).abs() < 1e-6);
    }

    #[tokio::test]
    async fn recall_empty_db() {
        let m = mem();
        let opts = agent::RecallOptions::default();
        let results = m.recall("anything", opts).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn recall_finds_relevant() {
        let m = mem();
        m.set("rust", "Rust is a systems programming language");
        m.set("python", "Python is a scripting language");
        m.set("cooking", "How to make pasta carbonara");

        let opts = agent::RecallOptions::default();
        let results = m.recall("programming language", opts).await.unwrap();
        assert!(!results.is_empty());
        // Should find entries containing "programming language".
        let keys: Vec<&str> = results.iter().map(|e| e.key.as_str()).collect();
        assert!(keys.contains(&"rust"));
    }

    #[tokio::test]
    async fn recall_respects_limit() {
        let m = mem();
        m.set("a", "rust programming language features");
        m.set("b", "rust programming best practices");
        m.set("c", "rust programming async runtime");

        let opts = agent::RecallOptions {
            limit: 1,
            ..Default::default()
        };
        let results = m.recall("rust programming", opts).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn recall_time_range_filter() {
        let m = mem();
        m.set("old", "rust is great");
        m.set("new", "rust is wonderful");

        // Both entries were created "now". Use a time range that excludes them.
        let opts = agent::RecallOptions {
            time_range: Some((0, 1)),
            ..Default::default()
        };
        let results = m.recall("rust", opts).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn compile_relevant_formats_xml() {
        let m = mem();
        m.set("user", "Prefers short answers about rust");
        m.set("persona", "You are a rust expert");

        let compiled = m.compile_relevant("rust").await;
        assert!(compiled.contains("<memory>"));
        assert!(compiled.contains("</memory>"));
        // Should contain at least one entry about rust.
        assert!(compiled.contains("rust"));
    }

    #[tokio::test]
    async fn compile_relevant_empty() {
        let m = mem();
        let compiled = m.compile_relevant("anything").await;
        assert!(compiled.is_empty());
    }
}
