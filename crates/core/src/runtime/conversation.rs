//! Conversation — container with append-only JSONL persistence.
//!
//! Files are organized as `sessions/{agent}_{sender}_{seq}.jsonl`.
//! After `set_title`, renamed to `{agent}_{sender}_{seq}_{title_slug}.jsonl`.
//!
//! Append-only. Compact markers (`{"compact":"..."}`) separate archived
//! history from the working context. Loading reads from the last compact
//! marker forward.

use crate::{
    AgentEvent, AgentStep,
    model::{Message, Usage},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::Instant,
};

/// Conversation metadata — first line of a JSONL conversation file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMeta {
    pub agent: String,
    pub created_by: String,
    pub created_at: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub uptime_secs: u64,
}

/// A JSONL line: message, compact marker (archive boundary), or trace event.
///
/// Variant order matters: `untagged` tries top-to-bottom. `Compact` and
/// `Event` are discriminated by required fields (`compact` / `event`) and
/// fail fast on other shapes, so `Message` (the catch-all) must be last.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum ConversationLine {
    Compact {
        compact: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        title: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        archived_at: String,
    },
    Event(EventLine),
    Message(Message),
}

/// A trace entry persisted to the conversation JSONL alongside messages.
///
/// Captures the *how* of agent execution (which tools ran, how long they
/// took, why the agent stopped, what it cost) — information that doesn't
/// fit in the message stream itself but is invaluable for debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum EventLine {
    /// One round of tool calls dispatched by the model.
    ToolStart {
        calls: Vec<ToolCallTrace>,
        ts: String,
    },
    /// A single tool call completed. Correlated to `ToolStart` via `call_id`.
    ToolResult {
        call_id: String,
        duration_ms: u64,
        ts: String,
    },
    /// Agent run finished — final metadata and token usage.
    Done {
        model: String,
        iterations: usize,
        stop_reason: String,
        usage: Usage,
        ts: String,
    },
    /// User steered the agent mid-stream.
    UserSteered { content: String, ts: String },
}

/// Compact tool call info for [`EventLine::ToolStart`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallTrace {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub arguments: String,
}

impl EventLine {
    /// Build a trace entry from an [`AgentEvent`]. Returns `None` for events
    /// that don't carry useful trace information (deltas, internal markers,
    /// duplicates of what messages already capture).
    pub fn from_agent_event(event: &AgentEvent) -> Option<Self> {
        let ts = chrono::Utc::now().to_rfc3339();
        match event {
            AgentEvent::ToolCallsStart(calls) => Some(Self::ToolStart {
                calls: calls
                    .iter()
                    .map(|c| ToolCallTrace {
                        id: c.id.clone(),
                        name: c.function.name.to_string(),
                        arguments: c.function.arguments.clone(),
                    })
                    .collect(),
                ts,
            }),
            AgentEvent::ToolResult {
                call_id,
                duration_ms,
                ..
            } => Some(Self::ToolResult {
                call_id: call_id.clone(),
                duration_ms: *duration_ms,
                ts,
            }),
            AgentEvent::Done(resp) => Some(Self::Done {
                model: resp.model.clone(),
                iterations: resp.iterations,
                stop_reason: resp.stop_reason.to_string(),
                usage: sum_step_usage(&resp.steps),
                ts,
            }),
            AgentEvent::UserSteered { content } => Some(Self::UserSteered {
                content: content.clone(),
                ts,
            }),
            _ => None,
        }
    }
}

/// Sum token usage across all steps of an agent run.
fn sum_step_usage(steps: &[AgentStep]) -> Usage {
    steps.iter().fold(Usage::default(), |mut acc, step| {
        let u = &step.response.usage;
        acc.prompt_tokens += u.prompt_tokens;
        acc.completion_tokens += u.completion_tokens;
        acc.total_tokens += u.total_tokens;
        if let Some(v) = u.prompt_cache_hit_tokens {
            *acc.prompt_cache_hit_tokens.get_or_insert(0) += v;
        }
        if let Some(v) = u.prompt_cache_miss_tokens {
            *acc.prompt_cache_miss_tokens.get_or_insert(0) += v;
        }
        acc
    })
}

/// A compaction archive segment — a titled snapshot of past conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveSegment {
    /// Short title derived from the compact summary.
    pub title: String,
    /// The compact summary text.
    pub summary: String,
    /// When this segment was archived.
    pub archived_at: String,
}

/// A conversation tied to a specific agent.
#[derive(Debug, Clone)]
pub struct Conversation {
    /// Unique conversation identifier (monotonic counter, runtime-only).
    pub id: u64,
    /// Name of the agent this conversation is bound to.
    pub agent: String,
    /// Conversation history (the working context for the LLM).
    pub history: Vec<Message>,
    /// Origin of this conversation (e.g. "user", "tg:12345").
    pub created_by: String,
    /// Conversation title (set by the `set_title` tool).
    pub title: String,
    /// Accumulated active time in seconds (persisted to meta).
    pub uptime_secs: u64,
    /// When this conversation was loaded/created in this process.
    pub created_at: Instant,
    /// Path to the JSONL persistence file.
    pub file_path: Option<PathBuf>,
}

impl Conversation {
    /// Create a new conversation with an empty history.
    pub fn new(id: u64, agent: impl Into<String>, created_by: impl Into<String>) -> Self {
        Self {
            id,
            agent: agent.into(),
            history: Vec::new(),
            created_by: created_by.into(),
            title: String::new(),
            uptime_secs: 0,
            created_at: Instant::now(),
            file_path: None,
        }
    }

    /// Ensure the JSONL file exists, creating it on first call.
    ///
    /// No-op if the file was already created or loaded from disk.
    pub fn ensure_file(&mut self) {
        if self.file_path.is_some() {
            return;
        }
        self.init_file(&crate::paths::CONVERSATIONS_DIR);
    }

    /// Initialize a new JSONL file in the flat conversations directory.
    ///
    /// Creates `{conversations_dir}/{agent}_{sender}_{seq}.jsonl` with
    /// seq auto-incremented globally per identity.
    pub fn init_file(&mut self, conversations_dir: &Path) {
        let _ = fs::create_dir_all(conversations_dir);

        let slug = sender_slug(&self.created_by);
        let prefix = format!("{}_{slug}_", self.agent);
        let seq = next_seq(conversations_dir, &prefix);
        let filename = format!("{prefix}{seq}.jsonl");
        let path = conversations_dir.join(filename);

        let meta = ConversationMeta {
            agent: self.agent.clone(),
            created_by: self.created_by.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            title: String::new(),
            uptime_secs: self.uptime_secs,
        };

        match OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
        {
            Ok(mut f) => {
                if let Ok(json) = serde_json::to_string(&meta) {
                    let _ = writeln!(f, "{json}");
                }
                self.file_path = Some(path);
            }
            Err(e) => tracing::warn!("failed to create conversation file: {e}"),
        }
    }

    /// Set the conversation title and rename the file to include the title slug.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();

        let Some(ref old_path) = self.file_path else {
            return;
        };

        // Rewrite meta line with the title.
        self.rewrite_meta();

        // Rename file: insert title slug before `.jsonl`.
        let title_slug = sender_slug(title);
        if title_slug.is_empty() {
            return;
        }
        let old_name = old_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let new_name = format!("{old_name}_{title_slug}.jsonl");
        let new_path = old_path.with_file_name(new_name);
        if fs::rename(old_path, &new_path).is_ok() {
            self.file_path = Some(new_path);
        }
    }

    /// Rewrite the meta line (first line) of the JSONL file.
    pub fn rewrite_meta(&self) {
        let Some(ref path) = self.file_path else {
            return;
        };
        let Ok(content) = fs::read_to_string(path) else {
            return;
        };
        let meta = ConversationMeta {
            agent: self.agent.clone(),
            created_by: self.created_by.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            title: self.title.clone(),
            uptime_secs: self.uptime_secs,
        };
        let Ok(meta_json) = serde_json::to_string(&meta) else {
            return;
        };
        // Replace only the first line.
        let rest = content.find('\n').map(|i| &content[i..]).unwrap_or("");
        let new_content = format!("{meta_json}{rest}");
        let _ = fs::write(path, new_content);
    }

    /// Append messages to the JSONL file. Skips auto-injected messages.
    pub fn append_messages(&self, messages: &[Message]) {
        let Some(ref path) = self.file_path else {
            return;
        };
        let mut file = match OpenOptions::new().append(true).open(path) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("failed to open conversation file for append: {e}");
                return;
            }
        };
        for msg in messages {
            if msg.auto_injected {
                continue;
            }
            if let Ok(json) = serde_json::to_string(msg) {
                let _ = writeln!(file, "{json}");
            }
        }
    }

    /// Append trace event entries to the JSONL file.
    ///
    /// Events sit alongside messages and compact markers but are skipped
    /// when reconstructing the LLM working context (see [`Self::load_context`]).
    ///
    /// Trace persistence is best-effort: events are buffered for the whole
    /// run and flushed alongside messages so file order matches chronological
    /// order. A daemon crash mid-run will lose the trace for that run; the
    /// messages themselves are likewise only persisted at run end.
    pub fn append_events(&self, events: &[EventLine]) {
        if events.is_empty() {
            return;
        }
        let Some(ref path) = self.file_path else {
            return;
        };
        let mut file = match OpenOptions::new().append(true).open(path) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("failed to open conversation file for events: {e}");
                return;
            }
        };
        for event in events {
            let line = ConversationLine::Event(event.clone());
            if let Ok(json) = serde_json::to_string(&line) {
                let _ = writeln!(file, "{json}");
            }
        }
    }

    /// Append a compact marker to the JSONL file.
    ///
    /// The marker doubles as an archive boundary: it stores the summary,
    /// a title derived from the first sentence, and a timestamp.
    pub fn append_compact(&self, summary: &str) {
        let Some(ref path) = self.file_path else {
            return;
        };
        let mut file = match OpenOptions::new().append(true).open(path) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("failed to open conversation file for compact: {e}");
                return;
            }
        };
        let line = ConversationLine::Compact {
            compact: summary.to_string(),
            title: compact_title(summary),
            archived_at: chrono::Utc::now().to_rfc3339(),
        };
        if let Ok(json) = serde_json::to_string(&line) {
            let _ = writeln!(file, "{json}");
        }
    }

    /// Load the working context from a JSONL conversation file.
    ///
    /// Reads from the last `{"compact":"..."}` marker forward. If no compact
    /// marker exists, loads all messages.
    pub fn load_context(path: &Path) -> anyhow::Result<(ConversationMeta, Vec<Message>)> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let meta_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("empty conversation file"))??;
        let meta: ConversationMeta = serde_json::from_str(&meta_line)?;

        let mut all_lines: Vec<String> = Vec::new();
        let mut last_compact_idx: Option<usize> = None;

        for line in lines {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if line.contains("\"compact\"")
                && let Ok(ConversationLine::Compact { .. }) = serde_json::from_str(&line)
            {
                last_compact_idx = Some(all_lines.len());
            }
            all_lines.push(line);
        }

        let context_start = last_compact_idx.unwrap_or_default();

        let mut messages = Vec::new();
        for (i, line) in all_lines[context_start..].iter().enumerate() {
            if i == 0
                && last_compact_idx.is_some()
                && let Ok(ConversationLine::Compact { compact, .. }) = serde_json::from_str(line)
            {
                messages.push(Message::user(&compact));
                continue;
            }
            // Skip event traces — they're not part of the LLM working context.
            match serde_json::from_str::<ConversationLine>(line) {
                Ok(ConversationLine::Message(msg)) => messages.push(msg),
                Ok(ConversationLine::Event(_) | ConversationLine::Compact { .. }) => {}
                Err(e) => tracing::warn!("skipping unparsable conversation line: {e}"),
            }
        }

        Ok((meta, messages))
    }

    /// Load all archive segments from a JSONL conversation file.
    ///
    /// Each compact marker in the file becomes an `ArchiveSegment` with
    /// title, summary, and timestamp. Returns segments in chronological order.
    pub fn load_archives(path: &Path) -> anyhow::Result<Vec<ArchiveSegment>> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut archives = Vec::new();

        for line in reader.lines().skip(1) {
            let line = line?;
            if line.contains("\"compact\"")
                && let Ok(ConversationLine::Compact {
                    compact,
                    title,
                    archived_at,
                }) = serde_json::from_str(&line)
            {
                archives.push(ArchiveSegment {
                    title,
                    summary: compact,
                    archived_at,
                });
            }
        }

        Ok(archives)
    }
}

/// Find the latest conversation file for an (agent, created_by) identity.
///
/// Scans the flat conversations directory for files matching the identity prefix
/// and returns the one with the highest seq number.
pub fn find_latest_conversation(
    conversations_dir: &Path,
    agent: &str,
    created_by: &str,
) -> Option<PathBuf> {
    let slug = sender_slug(created_by);
    let prefix = format!("{agent}_{slug}_");

    let mut best: Option<(u32, PathBuf)> = None;

    for entry in fs::read_dir(conversations_dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let name = path.file_name()?.to_str()?;
        if !name.starts_with(&prefix) || !name.ends_with(".jsonl") {
            continue;
        }
        let after_prefix = &name[prefix.len()..];
        let seq_str = after_prefix.split(|c: char| !c.is_ascii_digit()).next()?;
        let seq: u32 = seq_str.parse().ok()?;
        if best.as_ref().is_none_or(|(best_seq, _)| seq > *best_seq) {
            best = Some((seq, path));
        }
    }

    best.map(|(_, path)| path)
}

/// Compute the next seq number for a given prefix in a directory.
fn next_seq(dir: &Path, prefix: &str) -> u32 {
    let max = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name();
            let name = name.to_str()?;
            if !name.starts_with(prefix) || !name.ends_with(".jsonl") {
                return None;
            }
            let after_prefix = &name[prefix.len()..];
            let seq_str = after_prefix.split(|c: char| !c.is_ascii_digit()).next()?;
            seq_str.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0);
    max + 1
}

/// Derive a short title from a compact summary.
///
/// Takes the first sentence (up to the first `.`, `!`, or `?`) and caps
/// at 60 characters. Falls back to the first 60 chars if no sentence
/// boundary is found.
fn compact_title(summary: &str) -> String {
    let end = summary
        .find(['.', '!', '?'])
        .map(|i| i + 1)
        .unwrap_or(summary.len())
        .min(60);
    let title = summary[..summary.floor_char_boundary(end)].trim();
    title.to_string()
}

/// Sanitize a string into a filesystem-safe slug.
pub fn sender_slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
