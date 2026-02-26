//! Walrus gateway binary entry point.
//!
//! Resolves the global config directory and delegates to `gateway::serve()`.

use anyhow::Result;
use tokio::signal;
use tracing_subscriber::EnvFilter;
use walrus_gateway::{GatewayConfig, config};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config_dir = config::global_config_dir();
    let bind = GatewayConfig::load(&config_dir.join("gateway.toml"))?.bind_address();
    let handle = walrus_gateway::serve(&config_dir, &bind).await?;

    signal::ctrl_c().await?;
    tracing::info!("received ctrl-c, shutting down");
    handle.shutdown().await?;
    tracing::info!("gateway shut down");
    Ok(())
}
