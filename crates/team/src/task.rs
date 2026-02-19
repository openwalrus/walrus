//! Task types for leader ↔ worker communication.

/// A task delegated from a leader to a worker.
#[derive(Debug, Clone)]
pub struct Task {
    /// The input text / instruction for the worker.
    pub input: String,
}

/// The result of a completed task.
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// The worker's text response.
    pub output: String,
}
