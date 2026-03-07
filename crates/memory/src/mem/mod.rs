//! In-memory implementation of the Memory trait.

use std::sync::{Arc, Mutex};
use wcore::Memory;

/// In-memory store backed by `Arc<Mutex<Vec<(String, String)>>>`.
///
/// `Clone` is cheap — clones share the same underlying storage.
#[derive(Default, Debug, Clone)]
pub struct InMemory {
    entries: Arc<Mutex<Vec<(String, String)>>>,
}

impl InMemory {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a store pre-populated with entries.
    pub fn with_entries(entries: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            entries: Arc::new(Mutex::new(entries.into_iter().collect())),
        }
    }
}

impl Memory for InMemory {
    fn get(&self, key: &str) -> Option<String> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    fn entries(&self) -> Vec<(String, String)> {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn set(&self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        let key = key.into();
        let value = value.into();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = entries.iter_mut().find(|(k, _)| *k == key) {
            Some(std::mem::replace(&mut existing.1, value))
        } else {
            entries.push((key, value));
            None
        }
    }

    fn remove(&self, key: &str) -> Option<String> {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let idx = entries.iter().position(|(k, _)| k == key)?;
        Some(entries.remove(idx).1)
    }
}
