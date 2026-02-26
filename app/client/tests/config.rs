//! Tests for walrus-client configuration and builder.

use walrus_client::{ClientConfig, WalrusClient};

#[test]
fn client_config_defaults() {
    let config = ClientConfig::default();
    assert_eq!(config.gateway_url.as_str(), "ws://127.0.0.1:6688/ws");
    assert!(config.auth_token.is_none());
}

#[test]
fn client_builder() {
    let client = WalrusClient::new(ClientConfig::default())
        .gateway_url("ws://example.com/ws")
        .auth_token("secret-token");

    assert_eq!(client.config().gateway_url.as_str(), "ws://example.com/ws");
    assert_eq!(client.config().auth_token.as_deref(), Some("secret-token"));
}
