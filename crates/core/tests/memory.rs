//! Tests for the Memory trait and InMemory implementation.

use compact_str::CompactString;
use walrus_core::{Agent, InMemory, Memory, MemoryEntry, RecallOptions, with_memory};

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
