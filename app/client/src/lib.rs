//! Walrus client library — Unix domain socket client for connecting to a
//! walrus-gateway. Used by walrus-cli and other platform clients.

use std::path::PathBuf;
pub use connection::Connection;

pub mod connection;

/// Client configuration for connecting to a walrus-gateway.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Gateway Unix domain socket path.
    pub socket_path: PathBuf,
    /// Optional authentication token.
    pub auth_token: Option<compact_str::CompactString>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
            auth_token: None,
        }
    }
}

/// Default socket path: `~/.config/walrus/walrus.sock`.
fn default_socket_path() -> PathBuf {
    dirs::config_dir()
        .expect("no platform config directory")
        .join("walrus")
        .join("walrus.sock")
}

/// Unix domain socket client for the walrus-gateway.
///
/// Holds configuration. Call [`WalrusClient::connect`] to establish a
/// connection.
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

    /// Set the gateway socket path.
    pub fn socket_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.socket_path = path.into();
        self
    }

    /// Set the authentication token.
    pub fn auth_token(mut self, token: impl Into<compact_str::CompactString>) -> Self {
        self.config.auth_token = Some(token.into());
        self
    }

    /// Connect to the gateway and return a [`Connection`].
    pub async fn connect(&self) -> anyhow::Result<Connection> {
        Connection::connect(&self.config.socket_path).await
    }
}
