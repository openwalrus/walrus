//! Walrus client library — Unix domain socket client for connecting to a
//! walrus-gateway. Used by walrus-cli and other platform clients.

pub use connection::Connection;
use std::path::PathBuf;

pub mod connection;

/// Client configuration for connecting to a walrus-gateway.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Gateway Unix domain socket path.
    pub socket_path: PathBuf,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
        }
    }
}

/// Default socket path: `~/.walrus/walrus.sock`.
fn default_socket_path() -> PathBuf {
    dirs::home_dir()
        .expect("no home directory")
        .join(".walrus")
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

    /// Connect to the gateway and return a [`Connection`].
    pub async fn connect(&self) -> anyhow::Result<Connection> {
        Connection::connect(&self.config.socket_path).await
    }
}
