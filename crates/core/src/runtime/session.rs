//! Session — lightweight history container for agent conversations.

use crate::model::Message;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Instant, SystemTime},
};

/// Session metadata written as the first line of a JSONL session file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub agent: String,
    pub created_by: String,
    pub created_at: String,
}

/// A conversation session tied to a specific agent.
///
/// Sessions own the conversation history and are stored behind
/// `Arc<Mutex<Session>>` in the runtime. Multiple sessions can
/// reference the same agent — each with independent history.
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier (monotonic counter).
    pub id: u64,
    /// Name of the agent this session is bound to.
    pub agent: String,
    /// Conversation history (user/assistant/tool messages).
    pub history: Vec<Message>,
    /// Origin of this session (e.g. "user", "telegram:12345", agent name).
    pub created_by: String,
    /// When this session was created.
    pub created_at: Instant,
    /// Path to the JSONL persistence file (set when persistence is enabled).
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

    /// Initialize a JSONL persistence file in the given directory.
    ///
    /// Writes the metadata header line and sets `self.file_path`.
    pub fn init_file(&mut self, sessions_dir: &Path) {
        let _ = fs::create_dir_all(sessions_dir);
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("{}_{ts}_{}.jsonl", self.agent, self.id);
        let path = sessions_dir.join(filename);

        let meta = SessionMeta {
            agent: self.agent.clone(),
            created_by: self.created_by.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
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

    /// Persist the full session history to the JSONL file.
    ///
    /// Overwrites the file with the current metadata + all non-auto-injected
    /// messages. No-op if `file_path` is not set.
    pub fn persist(&self) {
        let Some(ref path) = self.file_path else {
            return;
        };

        let meta = SessionMeta {
            agent: self.agent.clone(),
            created_by: self.created_by.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let mut file = match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
        {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("failed to persist session {}: {e}", self.id);
                return;
            }
        };

        if let Ok(json) = serde_json::to_string(&meta) {
            let _ = writeln!(file, "{json}");
        }

        for msg in &self.history {
            if msg.auto_injected {
                continue;
            }
            if let Ok(json) = serde_json::to_string(msg) {
                let _ = writeln!(file, "{json}");
            }
        }
    }
}
