//! Gateway server command.

use crate::config::resolve_config;
use anyhow::Result;
use clap::Args;

/// Start the gateway server.
#[derive(Args, Debug)]
pub struct Serve {
    /// Bind address (host:port). Defaults to gateway.toml server config.
    #[arg(long)]
    pub url: Option<std::net::SocketAddr>,
}

impl Serve {
    /// Load config, build runtime, and start the axum server.
    pub async fn run(self) -> Result<()> {
        let config = resolve_config()?;
        let config_dir = gateway::config::global_config_dir();

        let bind = self
            .url
            .map(|a| a.to_string())
            .unwrap_or_else(|| config.bind_address());

        let handle = gateway::serve_with_config(&config, &config_dir, &bind).await?;

        tokio::signal::ctrl_c().await?;
        tracing::info!("received ctrl-c, shutting down");
        handle.shutdown().await?;
        tracing::info!("gateway shut down");
        Ok(())
    }
}
