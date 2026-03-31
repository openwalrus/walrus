//! Gateway clients for cross-framework benchmarks.
//!
//! Each gateway sends a task prompt to a framework and collects the response.

pub mod crabtalk;
pub mod hermes;
pub mod openclaw;
pub mod opencode;

use crate::task::Task;
use std::{
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

/// Result of running a task through a framework.
pub struct TaskResult {
    /// Whether the task completed successfully.
    pub success: bool,
    /// The framework's text response.
    pub response: String,
    /// Wall-clock time in milliseconds.
    pub wall_clock_ms: u64,
}

/// Common interface for sending tasks to agent frameworks.
pub trait Gateway {
    /// Send a task prompt and collect the response. Blocking (for Criterion).
    fn run_task(&self, rt: &tokio::runtime::Runtime, task: &Task) -> TaskResult;
}

/// Check if a TCP port is reachable (2s timeout).
pub fn check_reachable(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok()
}

/// Time a future and wrap the result in a TaskResult.
pub async fn timed<F, Fut>(f: F) -> TaskResult
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let start = Instant::now();
    let result = f().await;
    let wall_clock_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(response) => TaskResult {
            success: true,
            response,
            wall_clock_ms,
        },
        Err(e) => TaskResult {
            success: false,
            response: e,
            wall_clock_ms,
        },
    }
}
