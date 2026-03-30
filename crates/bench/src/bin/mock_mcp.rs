//! Standalone mock MCP server for cross-framework benchmarks.
//!
//! Usage:
//!   cargo run -p crabtalk-bench --bin mock-mcp            # random port
//!   cargo run -p crabtalk-bench --bin mock-mcp -- 9090    # fixed port

use crabtalk_bench::{mock_mcp, task};

#[tokio::main]
async fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let tasks = task::tasks();
    let (addr, _handle) = mock_mcp::start_on(port, &tasks).await;
    eprintln!("mock MCP server listening on http://{addr}/mcp");
    eprintln!(
        "tools: {}",
        tasks
            .iter()
            .flat_map(|t| &t.tools)
            .map(|t| t.name)
            .collect::<Vec<_>>()
            .join(", ")
    );
    eprintln!("press ctrl-c to stop");
    tokio::signal::ctrl_c().await.unwrap();
}
