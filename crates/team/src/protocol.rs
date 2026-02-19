//! Transport abstraction for leader ↔ worker communication.

use crate::task::{Task, TaskResult};
use anyhow::Result;

/// Communication protocol between leader and worker agents.
///
/// Implementations handle how a task reaches a worker and how
/// the result comes back. The leader never knows whether the
/// worker is in-process or across the network.
pub trait Protocol: Send + Sync + 'static {
    /// Send a task to a worker and await the result.
    fn call(&self, task: Task) -> impl Future<Output = Result<TaskResult>>;
}
