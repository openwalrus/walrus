//! `walrus daemon` — start the walrus daemon in the foreground.

use anyhow::Result;
use clap::Args;
use daemon::{Daemon as WalrusDaemon, config};

/// Start the walrus daemon in the foreground.
#[derive(Args, Debug)]
pub struct Daemon;

impl Daemon {
    /// Run the daemon, blocking until Ctrl-C.
    pub async fn run(self) -> Result<()> {
        if !config::GLOBAL_CONFIG_DIR.exists() {
            config::scaffold_config_dir(&config::GLOBAL_CONFIG_DIR)?;
            tracing::info!(
                "created config directory at {}",
                config::GLOBAL_CONFIG_DIR.display()
            );
        }

        let handle = WalrusDaemon::start(&config::GLOBAL_CONFIG_DIR).await?;

        // Spawn transports using the daemon's event sender.
        let (socket_path, socket_join) =
            daemon::setup_socket(&handle.shutdown_tx, &handle.event_tx)?;
        tracing::info!("walrusd listening on {}", socket_path.display());
        daemon::setup_channels(&handle.config, &handle.event_tx).await;

        tokio::signal::ctrl_c().await?;
        tracing::info!("received ctrl-c, shutting down");
        handle.shutdown().await?;
        socket_join.await?;
        let _ = std::fs::remove_file(socket_path);
        tracing::info!("walrusd shut down");
        Ok(())
    }
}
