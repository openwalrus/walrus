//! Tests for the MemoryBackend enum dispatch.

use walrus_daemon::MemoryBackend;

#[test]
fn in_memory_backend_set_and_get() {
    use wcore::Memory;
    let backend = MemoryBackend::in_memory();
    assert!(backend.get("key").is_none());
    backend.set("key", "value");
    assert_eq!(backend.get("key").unwrap(), "value");
}

#[test]
fn in_memory_backend_entries() {
    use wcore::Memory;
    let backend = MemoryBackend::in_memory();
    backend.set("a", "1");
    backend.set("b", "2");
    let entries = backend.entries();
    assert_eq!(entries.len(), 2);
}

#[test]
fn in_memory_backend_remove() {
    use wcore::Memory;
    let backend = MemoryBackend::in_memory();
    backend.set("key", "value");
    let old = backend.remove("key");
    assert_eq!(old.unwrap(), "value");
    assert!(backend.get("key").is_none());
}

#[test]
fn sqlite_backend_set_and_get() {
    use wcore::Memory;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let backend = MemoryBackend::sqlite(path.to_str().unwrap()).unwrap();
    assert!(backend.get("key").is_none());
    backend.set("key", "value");
    assert_eq!(backend.get("key").unwrap(), "value");
}

#[test]
fn sqlite_backend_entries() {
    use wcore::Memory;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let backend = MemoryBackend::sqlite(path.to_str().unwrap()).unwrap();
    backend.set("a", "1");
    backend.set("b", "2");
    let entries = backend.entries();
    assert_eq!(entries.len(), 2);
}

#[test]
fn sqlite_backend_remove() {
    use wcore::Memory;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let backend = MemoryBackend::sqlite(path.to_str().unwrap()).unwrap();
    backend.set("key", "value");
    let old = backend.remove("key");
    assert_eq!(old.unwrap(), "value");
    assert!(backend.get("key").is_none());
}

#[tokio::test]
async fn in_memory_backend_store() {
    use wcore::Memory;
    let backend = MemoryBackend::in_memory();
    backend.store("key", "value").await.unwrap();
    assert_eq!(backend.get("key").unwrap(), "value");
}

#[tokio::test]
async fn sqlite_backend_store() {
    use wcore::Memory;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let backend = MemoryBackend::sqlite(path.to_str().unwrap()).unwrap();
    backend.store("key", "value").await.unwrap();
    assert_eq!(backend.get("key").unwrap(), "value");
}

#[tokio::test]
async fn in_memory_backend_compile_relevant() {
    use wcore::Memory;
    let backend = MemoryBackend::in_memory();
    backend.set("fact", "the sky is blue");
    let compiled = backend.compile_relevant("sky color").await;
    assert!(compiled.contains("the sky is blue"));
}
