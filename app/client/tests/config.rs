//! Tests for walrus-client configuration and builder.

use walrus_client::{ClientConfig, WalrusClient};

#[test]
fn client_config_defaults() {
    let config = ClientConfig::default();
    assert!(config.socket_path.ends_with("walrus/walrus.sock"));
}

#[test]
fn client_builder() {
    let client = WalrusClient::new(ClientConfig::default())
        .socket_path("/tmp/test.sock");

    assert_eq!(
        client.config().socket_path,
        std::path::PathBuf::from("/tmp/test.sock")
    );
}
