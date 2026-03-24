//! Session — conversation container with append-only JSONL persistence.
//!
//! Each session is identified by `(agent, created_by)` and maps to a single
//! JSONL file: `~/.crabtalk/sessions/{agent}_{sender_slug}.jsonl`.
//!
//! The file is append-only. Compact markers (`{"compact":"..."}`) separate
//! archived history from the working context. Loading reads from the last
//! compact marker forward.

use crate::model::Message;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::Instant,
};

/// Session metadata — first line of a JSONL session file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub agent: String,
    pub created_by: String,
    pub created_at: String,
}

/// A JSONL line that is either a message or a compact marker.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum SessionLine {
    Compact { compact: String },
    Message(Message),
}

/// A conversation session tied to a specific agent.
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier (monotonic counter, runtime-only).
    pub id: u64,
    /// Name of the agent this session is bound to.
    pub agent: String,
    /// Conversation history (the working context for the LLM).
    pub history: Vec<Message>,
    /// Origin of this session (e.g. "user", "tg:12345").
    pub created_by: String,
    /// When this session was loaded/created in this process.
    pub created_at: Instant,
    /// Path to the JSONL persistence file.
    pub file_path: Option<PathBuf>,
}

impl Session {
    /// Create a new session with an empty history.
    pub fn new(id: u64, agent: impl Into<String>, created_by: impl Into<String>) -> Self {
        Self {
            id,
            agent: agent.into(),
            history: Vec::new(),
            created_by: created_by.into(),
            created_at: Instant::now(),
            file_path: None,
        }
    }

    /// Initialize the JSONL file for this session using identity-based naming.
    ///
    /// If the file already exists, opens for append. If new, writes the meta
    /// header line.
    pub fn init_file(&mut self, sessions_dir: &Path) {
        let _ = fs::create_dir_all(sessions_dir);
        let slug = sender_slug(&self.created_by);
        let filename = format!("{}_{slug}.jsonl", self.agent);
        let path = sessions_dir.join(filename);

        if path.exists() {
            // File already exists — just set the path, don't truncate.
            self.file_path = Some(path);
            return;
        }

        let meta = SessionMeta {
            agent: self.agent.clone(),
            created_by: self.created_by.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
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
            Err(e) => tracing::warn!("failed to create session file: {e}"),
        }
    }

    /// Append messages to the JSONL file. Skips auto-injected messages.
    pub fn append_messages(&self, messages: &[Message]) {
        let Some(ref path) = self.file_path else {
            return;
        };
        let mut file = match OpenOptions::new().append(true).open(path) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("failed to open session file for append: {e}");
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

    /// Append a compact marker to the JSONL file.
    pub fn append_compact(&self, summary: &str) {
        let Some(ref path) = self.file_path else {
            return;
        };
        let mut file = match OpenOptions::new().append(true).open(path) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("failed to open session file for compact: {e}");
                return;
            }
        };
        let line = SessionLine::Compact {
            compact: summary.to_string(),
        };
        if let Ok(json) = serde_json::to_string(&line) {
            let _ = writeln!(file, "{json}");
        }
    }

    /// Load the working context from a JSONL session file.
    ///
    /// Reads from the last `{"compact":"..."}` marker forward. If no compact
    /// marker exists, loads all messages.
    pub fn load_context(path: &Path) -> anyhow::Result<(SessionMeta, Vec<Message>)> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // First line is meta.
        let meta_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("empty session file"))??;
        let meta: SessionMeta = serde_json::from_str(&meta_line)?;

        // Read all remaining lines, tracking the last compact position.
        let mut all_lines: Vec<String> = Vec::new();
        let mut last_compact_idx: Option<usize> = None;

        for line in lines {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            // Check if this is a compact marker.
            if line.contains("\"compact\"")
                && let Ok(SessionLine::Compact { .. }) = serde_json::from_str(&line)
            {
                last_compact_idx = Some(all_lines.len());
            }
            all_lines.push(line);
        }

        // Build context from the last compact marker forward.
        let context_start = last_compact_idx.unwrap_or_default();

        let mut messages = Vec::new();
        for (i, line) in all_lines[context_start..].iter().enumerate() {
            if i == 0 && last_compact_idx.is_some() {
                // First line after compact marker IS the compact line — convert to user message.
                if let Ok(SessionLine::Compact { compact }) = serde_json::from_str(line) {
                    messages.push(Message::user(&compact));
                    continue;
                }
            }
            if let Ok(msg) = serde_json::from_str::<Message>(line) {
                messages.push(msg);
            }
        }

        Ok((meta, messages))
    }
}

/// Sanitize a sender string into a filesystem-safe slug.
pub fn sender_slug(sender: &str) -> String {
    sender
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Compute the session file path for an (agent, sender) identity.
pub fn session_file_path(sessions_dir: &Path, agent: &str, created_by: &str) -> PathBuf {
    let slug = sender_slug(created_by);
    sessions_dir.join(format!("{agent}_{slug}.jsonl"))
}
