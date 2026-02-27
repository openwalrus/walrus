//! Gateway server command.

use crate::config::resolve_config;
use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

/// Start the gateway server.
#[derive(Args, Debug)]
pub struct Serve {
    /// Custom socket path. Defaults to gateway.toml server config.
    #[arg(long)]
    pub socket: Option<PathBuf>,
}

impl Serve {
    /// Load config, build runtime, and start the gateway server.
    pub async fn run(self) -> Result<()> {
        let config = resolve_config()?;
        let config_dir = gateway::config::global_config_dir();

        let socket_path = self.socket.as_deref();
        let handle =
            gateway::serve_with_config(&config, &config_dir, socket_path).await?;

        tokio::signal::ctrl_c().await?;
        tracing::info!("received ctrl-c, shutting down");
        handle.shutdown().await?;
        tracing::info!("gateway shut down");
        Ok(())
    }
}
