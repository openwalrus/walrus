//! Crabtalk Telegram gateway entry point.

use clap::Parser;
use crabtalk_telegram::cmd::{App, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let level = std::env::var("RUST_LOG")
        .ok()
        .map(
            |v| match v.rsplit('=').next().unwrap_or(&v).to_lowercase().as_str() {
                "trace" => tracing::Level::TRACE,
                "debug" => tracing::Level::DEBUG,
                "info" => tracing::Level::INFO,
                "error" => tracing::Level::ERROR,
                _ => tracing::Level::WARN,
            },
        )
        .unwrap_or(tracing::Level::WARN);
    tracing_subscriber::fmt().with_max_level(level).init();
    let app = App::parse();
    match app.command {
        Command::Serve { daemon, config } => {
            crabtalk_telegram::cmd::serve::run(&daemon, &config).await
        }
    }
}
