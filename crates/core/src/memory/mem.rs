//! In-memory implementation of the Memory trait.

use crate::memory::Memory;
use std::sync::Mutex;

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
