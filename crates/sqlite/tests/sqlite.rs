//! Tests for SqliteMemory.

use agent::{Embedder, Memory, RecallOptions};
use agent::MemoryEntry;
use walrus_sqlite::SqliteMemory;

/// Noop embedder for tests that don't need vector search.
struct NoopEmbedder;

impl Embedder for NoopEmbedder {
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
    let dir = std::env::temp_dir().join("walrus_test_schema");
    let _ = std::fs::remove_file(&dir);
    let _m = SqliteMemory::<NoopEmbedder>::open(&dir).unwrap();

    // Open a separate connection to inspect the schema.
    let conn = rusqlite::Connection::open(&dir).unwrap();

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

    let _ = std::fs::remove_file(&dir);
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
    let opts = RecallOptions::default();
    let results: Vec<MemoryEntry> = m.recall("anything", opts).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn recall_finds_relevant() {
    let m = mem();
    m.set("rust", "Rust is a systems programming language");
    m.set("python", "Python is a scripting language");
    m.set("cooking", "How to make pasta carbonara");

    let opts = RecallOptions::default();
    let results: Vec<MemoryEntry> = m.recall("programming language", opts).await.unwrap();
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

    let opts = RecallOptions {
        limit: 1,
        ..Default::default()
    };
    let results: Vec<MemoryEntry> = m.recall("rust programming", opts).await.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn recall_time_range_filter() {
    let m = mem();
    m.set("old", "rust is great");
    m.set("new", "rust is wonderful");

    // Both entries were created "now". Use a time range that excludes them.
    let opts = RecallOptions {
        time_range: Some((0, 1)),
        ..Default::default()
    };
    let results: Vec<MemoryEntry> = m.recall("rust", opts).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn compile_relevant_formats_xml() {
    let m = mem();
    m.set("user", "Prefers short answers about rust");
    m.set("persona", "You are a rust expert");

    let compiled: String = m.compile_relevant("rust").await;
    assert!(compiled.contains("<memory>"));
    assert!(compiled.contains("</memory>"));
    // Should contain at least one entry about rust.
    assert!(compiled.contains("rust"));
}

#[tokio::test]
async fn compile_relevant_empty() {
    let m = mem();
    let compiled: String = m.compile_relevant("anything").await;
    assert!(compiled.is_empty());
}
