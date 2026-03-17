//! Task registry — in-memory tracking of agent work units.
//!
//! [`TaskRegistry`] stores [`Task`] records with concurrency control,
//! parent/sub-task hierarchy, and inbox-based blocking for user approval.

use compact_str::CompactString;
use tokio::sync::{oneshot, watch};
use tokio::task::AbortHandle;
use tokio::time::Instant;

pub use registry::TaskRegistry;

mod registry;
pub(crate) mod tool;

/// Task execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Waiting for a concurrency slot.
    Queued,
    /// Actively running.
    InProgress,
    /// Blocked waiting for user approval.
    Blocked,
    /// Completed successfully.
    Finished,
    /// Completed with error.
    Failed,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Blocked => write!(f, "blocked"),
            Self::Finished => write!(f, "finished"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Pending user approval item — blocks task until resolved.
pub struct InboxItem {
    /// Description of what needs approval (tool name + args summary).
    pub question: String,
    /// Channel to send the user's response through.
    pub reply: oneshot::Sender<String>,
}

impl std::fmt::Debug for InboxItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InboxItem")
            .field("question", &self.question)
            .finish()
    }
}

/// A tracked unit of agent work.
pub struct Task {
    /// Unique task identifier.
    pub id: u64,
    /// Parent task ID for sub-task hierarchy.
    pub parent_id: Option<u64>,
    /// Session allocated for this task's execution.
    pub session_id: Option<u64>,
    /// Agent assigned to this task.
    pub agent: CompactString,
    /// Current execution status.
    pub status: TaskStatus,
    /// Origin of this task ("user" or agent name).
    pub created_by: CompactString,
    /// Human-readable task description / message.
    pub description: String,
    /// Final result content (set on Finished).
    pub result: Option<String>,
    /// Error message (set on Failed).
    pub error: Option<String>,
    /// Pending approval item (set when status is Blocked).
    pub blocked_on: Option<InboxItem>,
    /// Cumulative prompt tokens used.
    pub prompt_tokens: u64,
    /// Cumulative completion tokens used.
    pub completion_tokens: u64,
    /// When this task was created.
    pub created_at: Instant,
    /// Handle to abort the spawned execution task.
    pub abort_handle: Option<AbortHandle>,
    /// Watch channel for status change notifications (used by await_tasks).
    pub status_tx: watch::Sender<TaskStatus>,
}
