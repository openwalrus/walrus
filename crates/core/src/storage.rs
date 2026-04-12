//! Persistence trait and domain types.
//!
//! [`Storage`] is the unified persistence backend — one trait, one
//! implementation per backend. Domain types ([`MemoryEntry`],
//! [`SessionHandle`], [`Skill`]) live alongside the trait they serve.

use crate::{
    AgentConfig, AgentId, ManifestConfig, NodeConfig,
    model::HistoryEntry,
    runtime::conversation::{ArchiveSegment, ConversationMeta, EventLine},
};
use anyhow::{Result, bail};
use std::collections::BTreeMap;

// ── Storage trait ───────────────────────────────────────────────────

/// Unified persistence backend.
///
/// All read/write operations for memory, skills, sessions, and agents
/// live here. Implementations own their encoding and storage layout —
/// the trait speaks domain types only.
pub trait Storage: Send + Sync + 'static {
    // ── Memory ─────────────────────────────────────────────────────

    /// List all memory entries.
    fn list_memories(&self) -> Result<Vec<MemoryEntry>>;

    /// Load a memory entry by name.
    fn load_memory(&self, name: &str) -> Result<Option<MemoryEntry>>;

    /// Create or replace a memory entry.
    fn save_memory(&self, entry: &MemoryEntry) -> Result<()>;

    /// Delete a memory entry by name. Returns `true` if it existed.
    fn delete_memory(&self, name: &str) -> Result<bool>;

    /// Load the curated MEMORY.md index content.
    fn load_memory_index(&self) -> Result<Option<String>>;

    /// Overwrite the MEMORY.md index content.
    fn save_memory_index(&self, content: &str) -> Result<()>;

    // ── Skills (read-only — skills are discovered from disk, not
    //    created through the runtime) ───────────────────────────────

    /// List all available skills.
    fn list_skills(&self) -> Result<Vec<Skill>>;

    /// Load a skill by name. Returns `None` if not found.
    fn load_skill(&self, name: &str) -> Result<Option<Skill>>;

    // ── Sessions ───────────────────────────────────────────────────

    /// Create a new session. Returns an opaque handle.
    fn create_session(&self, agent: &str, created_by: &str) -> Result<SessionHandle>;

    /// Find the latest session for an (agent, created_by) identity.
    fn find_latest_session(&self, agent: &str, created_by: &str) -> Result<Option<SessionHandle>>;

    /// Load a session's meta and working-context history.
    fn load_session(&self, handle: &SessionHandle) -> Result<Option<SessionSnapshot>>;

    /// Load all archive segments (compact markers) for a session.
    fn load_session_archives(&self, handle: &SessionHandle) -> Result<Vec<ArchiveSegment>>;

    /// List all sessions.
    fn list_sessions(&self) -> Result<Vec<SessionSummary>>;

    /// Append history entries to a session.
    fn append_session_messages(
        &self,
        handle: &SessionHandle,
        entries: &[HistoryEntry],
    ) -> Result<()>;

    /// Append trace event entries.
    fn append_session_events(&self, handle: &SessionHandle, events: &[EventLine]) -> Result<()>;

    /// Append a compact marker (archive boundary).
    fn append_session_compact(&self, handle: &SessionHandle, summary: &str) -> Result<()>;

    /// Overwrite session metadata.
    fn update_session_meta(&self, handle: &SessionHandle, meta: &ConversationMeta) -> Result<()>;

    /// Delete a session entirely.
    fn delete_session(&self, handle: &SessionHandle) -> Result<bool>;

    // ── Agents ─────────────────────────────────────────────────────

    /// List all persisted agent configs (with prompts loaded).
    fn list_agents(&self) -> Result<Vec<AgentConfig>>;

    /// Load a single agent by ULID.
    fn load_agent(&self, id: &AgentId) -> Result<Option<AgentConfig>>;

    /// Load a single agent by name.
    fn load_agent_by_name(&self, name: &str) -> Result<Option<AgentConfig>>;

    /// Create or replace an agent config and prompt.
    fn upsert_agent(&self, config: &AgentConfig, prompt: &str) -> Result<()>;

    /// Delete an agent by ULID. Returns `true` if it existed.
    fn delete_agent(&self, id: &AgentId) -> Result<bool>;

    /// Rename an agent. The ULID stays stable.
    fn rename_agent(&self, id: &AgentId, new_name: &str) -> Result<bool>;

    // ── Manifest ───────────────────────────────────────────────────

    /// Load the local manifest (`local/CrabTalk.toml`).
    /// Returns a default (empty) manifest if the file doesn't exist.
    fn load_local_manifest(&self) -> Result<ManifestConfig>;

    /// Overwrite the local manifest.
    fn save_local_manifest(&self, manifest: &ManifestConfig) -> Result<()>;

    // ── Config ──────────────────────────────────────────────────────

    /// Load the node configuration (`config.toml`).
    fn load_config(&self) -> Result<NodeConfig>;

    /// Overwrite the node configuration.
    fn save_config(&self, config: &NodeConfig) -> Result<()>;

    /// Create the initial config directory structure if it doesn't exist.
    fn scaffold(&self) -> Result<()>;
}

// ── Memory ──────────────────────────────────────────────────────────

/// A single memory entry with YAML frontmatter metadata.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    /// Human-readable name. Primary key for the repo (slugified for fs).
    pub name: String,
    /// One-line description used for relevance scoring.
    pub description: String,
    /// Entry body (markdown content).
    pub content: String,
}

impl MemoryEntry {
    /// Parse an entry from its frontmatter-based file content.
    pub fn parse(raw: &str) -> Result<Self> {
        let raw = raw.replace("\r\n", "\n");
        let raw = raw.trim();
        if !raw.starts_with("---") {
            bail!("missing frontmatter opening ---");
        }

        let after_open = &raw[3..];
        let Some(close_pos) = after_open.find("\n---") else {
            bail!("missing frontmatter closing ---");
        };

        let frontmatter = &after_open[..close_pos];
        let content = after_open[close_pos + 4..].trim().to_owned();

        let mut name = None;
        let mut description = None;

        for line in frontmatter.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("name:") {
                name = Some(val.trim().to_owned());
            } else if let Some(val) = line.strip_prefix("description:") {
                description = Some(val.trim().to_owned());
            }
        }

        let Some(name) = name else {
            bail!("missing 'name' in frontmatter");
        };
        let description = description.unwrap_or_default();

        Ok(Self {
            name,
            description,
            content,
        })
    }

    /// Serialize to the frontmatter file format.
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        out.push_str("---\n");
        out.push_str(&format!("name: {}\n", self.name));
        out.push_str(&format!("description: {}\n", self.description));
        out.push_str("---\n\n");
        out.push_str(&self.content);
        out.push('\n');
        out
    }

    /// Text for BM25 scoring — description + content concatenated.
    pub fn search_text(&self) -> String {
        format!("{} {}", self.description, self.content)
    }
}

/// Convert a name to a filesystem-safe slug.
pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut prev_dash = true;

    for ch in name.chars() {
        if ch.is_alphanumeric() {
            for lc in ch.to_lowercase() {
                slug.push(lc);
            }
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        slug.push_str("entry");
    }

    slug
}

// ── Sessions ────────────────────────────────────────────────────────

/// Opaque handle identifying a persisted session. Created by the repo
/// on `create`, returned by `find_latest`. Callers pass it back to
/// append/load methods without interpreting the inner value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionHandle(String);

impl SessionHandle {
    /// Construct a handle from a repo-assigned identifier.
    pub fn new(slug: impl Into<String>) -> Self {
        Self(slug.into())
    }

    /// The raw identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Snapshot returned by [`Storage::load_session`] — meta +
/// working-context history, already replayed past the last compact
/// marker.
pub struct SessionSnapshot {
    pub meta: ConversationMeta,
    pub history: Vec<HistoryEntry>,
}

/// Summary returned by [`Storage::list_sessions`] for enumeration.
pub struct SessionSummary {
    pub handle: SessionHandle,
    pub meta: ConversationMeta,
}

// ── Skills ──────────────────────────────────────────────────────────

/// A named unit of agent behavior (agentskills.io format).
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata: BTreeMap<String, String>,
    pub allowed_tools: Vec<String>,
    pub body: String,
}
