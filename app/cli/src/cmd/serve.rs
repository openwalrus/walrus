//! Gateway server command.

use crate::config::resolve_config;
use anyhow::Result;
use clap::Args;
use std::sync::Arc;

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
        let runtime = gateway::build_runtime(&config, &config_dir).await?;

        let bind_addr = self
            .url
            .map(|a| a.to_string())
            .unwrap_or_else(|| config.bind_address());

        let authenticator = gateway::ApiKeyAuthenticator::from_config(&config.auth);
        let state = gateway::Gateway {
            runtime: Arc::new(runtime),
            sessions: Arc::new(gateway::SessionManager::new()),
            authenticator: Arc::new(authenticator),
        };

        let app = gateway::gateway::ws::router(state);
        let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
        tracing::info!("gateway listening on {bind_addr}");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        tracing::info!("gateway shut down");
        Ok(())
    }
}

/// Wait for ctrl-c signal for graceful shutdown.
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install ctrl-c handler");
    tracing::info!("received shutdown signal");
}
