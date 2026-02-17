//! In-memory key-value store.

use crate::Memory;

/// In-memory store backed by `Vec<(String, String)>`.
///
/// Useful for testing, CLI tools, and as a local cache for
/// persistent backends.
#[derive(Clone, Default, Debug)]
pub struct InMemory {
    entries: Vec<(String, String)>,
}

impl InMemory {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a store pre-populated with entries.
    pub fn with_entries(entries: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

impl Memory for InMemory {
    fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn entries(&self) -> &[(String, String)] {
        &self.entries
    }

    fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        let key = key.into();
        let value = value.into();
        if let Some(existing) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            Some(std::mem::replace(&mut existing.1, value))
        } else {
            self.entries.push((key, value));
            None
        }
    }

    fn remove(&mut self, key: &str) -> Option<String> {
        let idx = self.entries.iter().position(|(k, _)| k == key)?;
        Some(self.entries.remove(idx).1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut mem = InMemory::new();
        assert!(mem.get("user").is_none());

        mem.set("user", "likes rust");
        assert_eq!(mem.get("user").unwrap(), "likes rust");
    }

    #[test]
    fn upsert_returns_old() {
        let mut mem = InMemory::new();
        assert!(mem.set("user", "v1").is_none());

        let old = mem.set("user", "v2");
        assert_eq!(old.unwrap(), "v1");
        assert_eq!(mem.get("user").unwrap(), "v2");
    }

    #[test]
    fn remove_returns_value() {
        let mem = InMemory::with_entries([("a".into(), "1".into())]);
        let mut mem = mem;
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
        let mut mem = InMemory::new();
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
}
