//! Walrus client library — shared WebSocket client for connecting to a
//! walrus-gateway. Used by walrus-cli and other platform clients.

use compact_str::CompactString;
pub use connection::Connection;

pub mod connection;

/// Client configuration for connecting to a walrus-gateway.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Gateway WebSocket URL.
    pub gateway_url: CompactString,
    /// Optional authentication token.
    pub auth_token: Option<CompactString>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            gateway_url: CompactString::from("ws://127.0.0.1:3000/ws"),
            auth_token: None,
        }
    }
}

/// WebSocket client for the walrus-gateway.
///
/// Holds configuration. Call [`WalrusClient::connect`] to establish a
/// WebSocket connection (requires the connection module).
pub struct WalrusClient {
    config: ClientConfig,
}

impl WalrusClient {
    /// Create a new client with the given configuration.
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }

    /// Access the client configuration.
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Set the gateway URL.
    pub fn gateway_url(mut self, url: impl Into<CompactString>) -> Self {
        self.config.gateway_url = url.into();
        self
    }

    /// Set the authentication token.
    pub fn auth_token(mut self, token: impl Into<CompactString>) -> Self {
        self.config.auth_token = Some(token.into());
        self
    }

    /// Connect to the gateway and return a [`Connection`].
    pub async fn connect(&self) -> anyhow::Result<Connection> {
        Connection::connect(&self.config).await
    }
}
