//! Shared gateway serve entrypoint — used by the binary, CLI, and FFI crate.

use crate::{ApiKeyAuthenticator, GatewayConfig, SessionManager, gateway::Gateway};
use anyhow::Result;
use std::path::Path;
use tokio::sync::oneshot;

/// Handle returned by [`serve`] — holds the bound port and shutdown trigger.
pub struct ServeHandle {
    /// The port the gateway is listening on.
    pub port: u16,
    /// Send a value to trigger graceful shutdown.
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Join handle for the server task.
    join: Option<tokio::task::JoinHandle<Result<(), std::io::Error>>>,
}

impl ServeHandle {
    /// Trigger graceful shutdown and wait for the server to stop.
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            join.await??;
        }
        Ok(())
    }
}

/// Load config, build runtime, bind the axum server, and start serving.
///
/// Returns a [`ServeHandle`] with the bound port and a shutdown trigger.
/// The server runs in a spawned task — call `handle.shutdown()` to stop it.
pub async fn serve(config_dir: &Path, bind: &str) -> Result<ServeHandle> {
    let config_path = config_dir.join("gateway.toml");
    let config = GatewayConfig::load(&config_path)?;
    tracing::info!("loaded configuration from {}", config_path.display());
    serve_with_config(&config, config_dir, bind).await
}

/// Serve with an already-loaded config. Useful when the caller resolves
/// config separately (e.g. CLI with scaffold logic).
pub async fn serve_with_config(
    config: &GatewayConfig,
    config_dir: &Path,
    bind: &str,
) -> Result<ServeHandle> {
    use crate::gateway::ws;
    use std::sync::Arc;

    let runtime = crate::build_runtime(config, config_dir).await?;
    let authenticator = ApiKeyAuthenticator::from_config(&config.auth);

    let state = Gateway {
        runtime: Arc::new(runtime),
        sessions: Arc::new(SessionManager::new()),
        authenticator: Arc::new(authenticator),
    };

    let app = ws::router(state);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    let port = listener.local_addr()?.port();
    tracing::info!("gateway listening on {bind} (port {port})");

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let join = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
                tracing::info!("received shutdown signal");
            })
            .await
    });

    Ok(ServeHandle {
        port,
        shutdown_tx: Some(shutdown_tx),
        join: Some(join),
    })
}
