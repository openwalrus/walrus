//! walrusd — the walrus daemon.
//!
//! Resolves the global config directory, scaffolds on first run, and serves
//! the Unix domain socket.

use anyhow::Result;
use tokio::signal;
use tracing_subscriber::EnvFilter;
use walrus_gateway::config;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config_dir = config::global_config_dir();
    if !config_dir.exists() {
        config::scaffold_config_dir(&config_dir)?;
        tracing::info!("created config directory at {}", config_dir.display());
    }

    let handle = walrus_gateway::serve(&config_dir, None).await?;
    tracing::info!("walrusd listening on {}", handle.socket_path.display());

    signal::ctrl_c().await?;
    tracing::info!("received ctrl-c, shutting down");
    handle.shutdown().await?;
    tracing::info!("walrusd shut down");
    Ok(())
}
