//! Built-in memory — markdown file storage at `{config_dir}/memory/`.
//!
//! [`BuiltinMemory`] manages four storage areas: `memory.md` (agent notes),
//! `user.md` (user profile), `facts.toml` (structured facts), and `sessions/`
//! (compact summaries). File contents are cached in-memory with write-through
//! on modification. Thread-safe via `std::sync::RwLock`.

use crate::hook::system::MemoryConfig;
use std::{path::PathBuf, sync::RwLock};
use wcore::model::{Message, Role};

pub(crate) mod tool;

const MEMORY_PROMPT: &str = include_str!("../../../../prompts/memory.md");
const EXTRACT_FACTS_PROMPT: &str = include_str!("../../../../prompts/extract-facts.md");

/// In-memory cache of a single file's content.
struct FileCache {
    path: PathBuf,
    content: String,
}

impl FileCache {
    /// Load from disk, or empty string if file doesn't exist.
    fn load(path: PathBuf) -> Self {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        Self { path, content }
    }

    /// Append text and flush to disk, respecting a character limit.
    /// Returns `true` if the write succeeded.
    fn append(&mut self, text: &str, limit: usize) -> bool {
        if self.content.len() + text.len() > limit {
            return false;
        }
        if !self.content.is_empty() && !self.content.ends_with('\n') {
            self.content.push('\n');
        }
        self.content.push_str(text);
        self.flush()
    }

    /// Overwrite content and flush to disk, respecting a character limit.
    /// Returns `true` if the write succeeded.
    fn write(&mut self, text: &str, limit: usize) -> bool {
        if text.len() > limit {
            return false;
        }
        self.content = text.to_owned();
        self.flush()
    }

    /// Write content to disk.
    fn flush(&self) -> bool {
        std::fs::write(&self.path, &self.content).is_ok()
    }
}

pub struct BuiltinMemory {
    memory: RwLock<FileCache>,
    user: RwLock<FileCache>,
    facts: RwLock<FileCache>,
    sessions_dir: PathBuf,
    config: MemoryConfig,
}

impl BuiltinMemory {
    /// Open (or create) memory storage at the given directory.
    pub fn open(dir: PathBuf, config: MemoryConfig) -> Self {
        let sessions_dir = dir.join("sessions");
        std::fs::create_dir_all(&sessions_dir).ok();

        let memory = RwLock::new(FileCache::load(dir.join("memory.md")));
        let user = RwLock::new(FileCache::load(dir.join("user.md")));
        let facts = RwLock::new(FileCache::load(dir.join("facts.toml")));

        Self {
            memory,
            user,
            facts,
            sessions_dir,
            config,
        }
    }

    /// Substring search across memory.md, user.md, and facts.toml.
    /// Returns matching lines with source labels.
    pub fn recall(&self, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        let sources: &[(&str, &RwLock<FileCache>)] = &[
            ("memory", &self.memory),
            ("user", &self.user),
            ("facts", &self.facts),
        ];

        for (label, cache) in sources {
            let guard = cache.read().unwrap();
            for line in guard.content.lines() {
                if line.to_lowercase().contains(&query_lower) {
                    results.push(format!("[{label}] {line}"));
                }
            }
        }

        if results.is_empty() {
            "no matches found".to_owned()
        } else {
            results.join("\n")
        }
    }

    /// Append to memory.md, respecting `memory_limit`.
    pub fn write_memory(&self, content: &str) -> String {
        let mut guard = self.memory.write().unwrap();
        if guard.append(content, self.config.memory_limit) {
            "written to memory".to_owned()
        } else {
            format!(
                "memory limit reached ({} chars, {} used)",
                self.config.memory_limit,
                guard.content.len()
            )
        }
    }

    /// Write to user.md, respecting `user_limit`. Overwrites existing content.
    pub fn write_user(&self, content: &str) -> String {
        let mut guard = self.user.write().unwrap();
        if guard.write(content, self.config.user_limit) {
            "written to user profile".to_owned()
        } else {
            format!(
                "user profile limit reached ({} chars)",
                self.config.user_limit
            )
        }
    }

    /// Build XML blocks for system prompt injection.
    pub fn build_prompt(&self) -> String {
        let mut blocks = Vec::new();

        let mem = self.memory.read().unwrap();
        if !mem.content.is_empty() {
            blocks.push(format!("<memory>\n{}\n</memory>", mem.content));
        }

        let usr = self.user.read().unwrap();
        if !usr.content.is_empty() {
            blocks.push(format!("<user>\n{}\n</user>", usr.content));
        }

        let facts = self.facts.read().unwrap();
        if !facts.content.is_empty() {
            blocks.push(format!("<facts>\n{}\n</facts>", facts.content));
        }

        if blocks.is_empty() {
            String::new()
        } else {
            format!("\n\n{}\n\n{MEMORY_PROMPT}", blocks.join("\n\n"))
        }
    }

    /// Recall from last user message, return as injected message.
    pub fn before_run(&self, history: &[Message]) -> Vec<Message> {
        let last_user = history
            .iter()
            .rev()
            .find(|m| m.role == Role::User && !m.content.is_empty());

        let Some(msg) = last_user else {
            return Vec::new();
        };

        // Use the first few words as a recall query.
        let query: String = msg
            .content
            .split_whitespace()
            .take(8)
            .collect::<Vec<_>>()
            .join(" ");

        if query.is_empty() {
            return Vec::new();
        }

        let result = self.recall(&query);
        if result == "no matches found" {
            return Vec::new();
        }

        vec![Message {
            role: Role::User,
            content: format!("<recall>\n{result}\n</recall>"),
            ..Default::default()
        }]
    }

    /// Save a session summary after compaction. Runs synchronously.
    /// Spawns facts extraction as a background task.
    pub fn after_compact(&self, agent: &str, summary: &str) {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{agent}_{timestamp}.md");
        let path = self.sessions_dir.join(filename);
        if let Err(e) = std::fs::write(&path, summary) {
            tracing::warn!("failed to save session summary: {e}");
        }

        // Extract facts synchronously (simple heuristic, no LLM).
        self.extract_facts(summary);
    }

    /// Lightweight facts extraction from a summary string.
    /// Looks for "Name: Value", "Key = Value" patterns and appends to facts.toml.
    pub fn extract_facts(&self, summary: &str) {
        let mut new_facts = Vec::new();

        for line in summary.lines() {
            let trimmed = line.trim();
            // Skip empty lines and headings.
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Match "Key: Value" patterns (but not URLs like "https://").
            if let Some((key, value)) = trimmed.split_once(": ") {
                let key = key.trim();
                let value = value.trim();
                if !key.is_empty()
                    && !value.is_empty()
                    && !key.contains(' ')
                    && !key.contains('/')
                    && key.len() < 32
                {
                    let safe_key = key.to_lowercase().replace('-', "_");
                    new_facts.push(format!("{safe_key} = {value:?}"));
                }
            }
        }

        if new_facts.is_empty() {
            return;
        }

        let mut guard = self.facts.write().unwrap();
        let addition = new_facts.join("\n");
        if !guard.content.is_empty() && !guard.content.ends_with('\n') {
            guard.content.push('\n');
        }
        guard.content.push_str(&addition);
        if !guard.flush() {
            tracing::warn!("failed to write facts.toml");
        }
    }
}

// Suppress unused warning for the extract-facts prompt — will be used when
// LLM extraction is added.
const _: &str = EXTRACT_FACTS_PROMPT;
