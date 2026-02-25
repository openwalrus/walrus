//! MCP example — connect to multiple MCP servers and use their tools via REPL.
//!
//! Connects to two MCP servers simultaneously:
//! 1. `@playwright/mcp` — browser automation (navigate, click, type, snapshot)
//! 2. `@modelcontextprotocol/server-filesystem` — filesystem access
//!
//! All tools from both servers are registered into the runtime and available
//! to the LLM in a single REPL session.
//!
//! Requires DEEPSEEK_API_KEY and `npx` (Node.js 18+). Run with:
//! ```sh
//! cargo run -p walrus-runtime --example mcp -- /path/to/allowed/directory
//! ```

mod common;

use walrus_runtime::{McpBridge, prelude::*};

#[tokio::main]
async fn main() {
    common::init_tracing();

    let allowed_dir = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo run -p walrus-runtime --example mcp -- <allowed_directory>");
        eprintln!("  The filesystem MCP server will only access files in that directory.");
        std::process::exit(1);
    });

    let mut runtime = common::build_runtime();
    let bridge = McpBridge::new();

    // 1. Connect to Playwright MCP server (headless browser automation).
    let mut playwright_cmd = tokio::process::Command::new("npx");
    playwright_cmd.args(["@playwright/mcp@latest", "--headless"]);

    println!("Connecting to Playwright MCP server...");
    bridge
        .connect_stdio(playwright_cmd)
        .await
        .expect("failed to connect to Playwright MCP server (is npx/node installed?)");

    let playwright_tools = bridge.tools().await;
    println!("  Playwright: {} tools", playwright_tools.len());

    // 2. Connect to filesystem MCP server.
    let mut fs_cmd = tokio::process::Command::new("npx");
    fs_cmd.args([
        "-y",
        "@modelcontextprotocol/server-filesystem",
        &allowed_dir,
    ]);

    println!("Connecting to filesystem MCP server...");
    bridge
        .connect_stdio(fs_cmd)
        .await
        .expect("failed to connect to filesystem MCP server");

    // List all discovered tools from both servers.
    let all_tools = bridge.tools().await;
    let fs_count = all_tools.len() - playwright_tools.len();
    println!("  Filesystem: {} tools", fs_count);
    println!("\nAll MCP tools ({} total):", all_tools.len());
    for tool in &all_tools {
        println!("  - {}: {}", tool.name, tool.description);
    }

    // Wire all MCP tools into the runtime's tool registry.
    runtime.connect_mcp(bridge);
    runtime
        .register_mcp_tools()
        .await
        .expect("failed to register MCP tools");

    runtime.add_agent(
        Agent::new("assistant")
            .system_prompt(format!(
                "You are a helpful assistant with browser and filesystem capabilities.\n\
                 - Use Playwright tools to browse the web, navigate pages, and interact with elements.\n\
                 - Use filesystem tools to explore and read files in: {allowed_dir}"
            ))
            .tool("*"),
    );

    println!("\nMCP REPL — try asking:");
    println!("  'Go to https://example.com and tell me what you see'");
    println!("  'List the files in the directory'");
    println!("  'Read the contents of README.md'");
    println!("(type 'exit' to quit)");
    println!("---");
    common::repl(&mut runtime, "assistant").await;
}
