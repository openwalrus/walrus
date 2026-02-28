//! Shared gateway serve entrypoint — used by the binary and CLI.

use crate::{DaemonConfig, gateway::Gateway};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::sync::oneshot;

/// Handle returned by [`serve`] — holds the socket path and shutdown trigger.
pub struct ServeHandle {
    /// The Unix domain socket path the gateway is listening on.
    pub socket_path: PathBuf,
    /// Send a value to trigger graceful shutdown.
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Join handle for the server task.
    join: Option<tokio::task::JoinHandle<()>>,
}

impl ServeHandle {
    /// Trigger graceful shutdown and wait for the server to stop.
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            join.await?;
        }
        // Clean up the socket file.
        let _ = std::fs::remove_file(&self.socket_path);
        Ok(())
    }
}

/// Load config, build runtime, bind the Unix domain socket, and start serving.
///
/// Returns a [`ServeHandle`] with the socket path and a shutdown trigger.
pub async fn serve(config_dir: &Path) -> Result<ServeHandle> {
    let config_path = config_dir.join("walrus.toml");
    let config = DaemonConfig::load(&config_path)?;
    tracing::info!("loaded configuration from {}", config_path.display());
    serve_with_config(&config, config_dir).await
}

/// Serve with an already-loaded config. Useful when the caller resolves
/// config separately (e.g. CLI with scaffold logic).
pub async fn serve_with_config(config: &DaemonConfig, config_dir: &Path) -> Result<ServeHandle> {
    use crate::gateway::uds;
    use std::sync::Arc;

    let runtime = crate::build_runtime(config, config_dir).await?;

    let state = Gateway {
        runtime: Arc::new(runtime),
    };

    let resolved_path = crate::config::socket_path();

    // Ensure the parent directory exists.
    if let Some(parent) = resolved_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove stale socket file if present.
    if resolved_path.exists() {
        std::fs::remove_file(&resolved_path)?;
    }

    let listener = tokio::net::UnixListener::bind(&resolved_path)?;
    tracing::info!("gateway listening on {}", resolved_path.display());

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let join = tokio::spawn(uds::accept_loop(listener, state, shutdown_rx));

    Ok(ServeHandle {
        socket_path: resolved_path,
        shutdown_tx: Some(shutdown_tx),
        join: Some(join),
    })
}
