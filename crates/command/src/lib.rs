pub use crabtalk_command_codegen::command;

// Re-export common deps so templates only need `crabtalk-command`.
pub use anyhow;
pub use clap;
pub use futures_util;
pub use tokio;
pub use tracing;
pub use tracing_subscriber;
pub use wcore;

/// Shared entry point: init tracing from `RUST_LOG`, build a tokio runtime,
/// run the given async closure, and exit on error.
pub fn run<F, Fut>(f: F)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    if let Some(level) = std::env::var("RUST_LOG").ok().map(|v| {
        match v.rsplit('=').next().unwrap_or(&v).to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::WARN,
        }
    }) {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_max_level(level)
            .init();
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = rt.block_on(f()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

pub mod service;
pub use service::{Service, install, render_service_template, uninstall, view_logs};

#[cfg(feature = "mcp")]
pub use axum;
#[cfg(feature = "mcp")]
pub use service::{McpService, run_mcp};
#[cfg(feature = "mcp")]
pub use {schemars, serde_json};

#[cfg(feature = "client")]
pub use service::ClientService;
#[cfg(feature = "client")]
pub use transport;
