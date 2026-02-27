//! Walrus gateway binary entry point.
//!
//! Resolves the global config directory and delegates to `gateway::serve()`.

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
    let handle = walrus_gateway::serve(&config_dir, None).await?;

    signal::ctrl_c().await?;
    tracing::info!("received ctrl-c, shutting down");
    handle.shutdown().await?;
    tracing::info!("gateway shut down");
    Ok(())
}
