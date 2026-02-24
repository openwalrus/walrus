//! Structured knowledge memory for LLM agents.
//!
//! Memory is **not chat history**. It is structured knowledge — extracted
//! facts, user preferences, agent persona — that gets compiled into the
//! system prompt.
//!
//! # Usage
//!
//! ```rust,ignore
//! use cydonia_agent::{Agent, InMemory, Memory, with_memory};
//!
//! let mut memory = InMemory::new();
//! memory.set("user", "Prefers short answers.");
//!
//! let agent = Agent::new("anto").system_prompt("You are helpful.");
//! let agent = with_memory(agent, &memory);
//! ```

use crate::Agent;

/// Structured knowledge memory for LLM agents.
///
/// Implementations store named key-value pairs that get compiled
/// into the system prompt via [`compile()`](Memory::compile).
pub trait Memory: Clone + Send + Sync {
    /// Get the value for a key.
    fn get(&self, key: &str) -> Option<&str>;

    /// Get all key-value pairs.
    fn entries(&self) -> &[(String, String)];

    /// Set (upsert) a key-value pair. Returns the previous value if the key existed.
    fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> Option<String>;

    /// Remove a key. Returns the removed value if it existed.
    fn remove(&mut self, key: &str) -> Option<String>;

    /// Compile all entries into a string for system prompt injection.
    fn compile(&self) -> String {
        let entries = self.entries();
        if entries.is_empty() {
            return String::new();
        }

        let mut out = String::from("<memory>\n");
        for (key, value) in entries {
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
}

/// In-memory store backed by `Vec<(String, String)>`.
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
        let mut mem = InMemory::with_entries([("a".into(), "1".into())]);
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

    #[test]
    fn with_memory_appends() {
        let mut mem = InMemory::new();
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
}
