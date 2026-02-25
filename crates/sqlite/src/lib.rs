//! SQLite-backed memory for Walrus agents.
//!
//! Provides [`SqliteMemory`], a persistent [`Memory`](agent::Memory) implementation
//! using SQLite with FTS5 full-text search.
//!
//! All SQL lives in `sql/*.sql` files, loaded via `include_str!`.

use crate::utils::{mmr_rerank, now_unix};
use agent::{Embedder, MemoryEntry, RecallOptions};
use anyhow::Result;
use compact_str::CompactString;
use rusqlite::Connection;
use serde_json::Value;
use std::{path::Path, sync::Mutex};

mod memory;
mod sql;
mod utils;

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
        conn.execute_batch(sql::SCHEMA)?;
        Ok(())
    }

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

        let mut stmt = conn.prepare(sql::RECALL_FTS)?;

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
            sql::UPSERT_FULL,
            rusqlite::params![key, value, meta_json, now, emb_blob],
        )?;
        Ok(())
    }

    /// Get a full MemoryEntry for a key.
    pub fn get_entry(&self, key: &str) -> Option<MemoryEntry> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(sql::SELECT_ENTRY, [key], |row| {
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
