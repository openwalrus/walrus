//! Tests for GatewayRunner construction types.

/// Verify GatewayRunner is Send (required for async dispatch).
#[test]
fn gateway_runner_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<walrus_cli::runner::gateway::GatewayRunner>();
}
