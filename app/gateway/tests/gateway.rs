//! Gateway integration tests.

use walrus_gateway::GatewayConfig;

/// Verify that GatewayConfig default bind address is correct.
#[test]
fn default_bind_address() {
    let config = GatewayConfig::default();
    assert_eq!(config.bind_address(), "127.0.0.1:3000");
}
