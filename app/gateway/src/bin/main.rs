//! Walrus gateway binary entry point.
//!
//! Loads TOML configuration, delegates runtime construction to
//! `build_runtime`, and runs the axum server with graceful shutdown.

use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::EnvFilter;
use walrus_gateway::{
    ApiKeyAuthenticator, Gateway, GatewayConfig, SessionManager, build_runtime, config,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing from RUST_LOG (default: info).
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Resolve global config directory.
    let config_dir = config::global_config_dir();
    let config_path = config_dir.join("gateway.toml");
    let config = GatewayConfig::load(&config_path)?;
    tracing::info!("loaded configuration from {}", config_path.display());

    // Build runtime with full config (memory, provider, skills, MCP, agents).
    let runtime = build_runtime(&config, &config_dir).await?;
    let bind_address = config.bind_address();

    // Initialize authenticator from config api keys.
    let authenticator = ApiKeyAuthenticator::from_config(&config.auth);

    // Build app state.
    let state = Gateway {
        runtime: Arc::new(runtime),
        sessions: Arc::new(SessionManager::new()),
        authenticator: Arc::new(authenticator),
    };

    // Build axum router.
    let app = walrus_gateway::gateway::ws::router(state);

    // Bind and serve.
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!("gateway listening on {bind_address}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("gateway shut down");
    Ok(())
}

/// Wait for ctrl-c signal for graceful shutdown.
async fn shutdown_signal() {
    signal::ctrl_c()
        .await
        .expect("failed to install ctrl-c handler");
    tracing::info!("received shutdown signal");
}
