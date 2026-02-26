//! Tests for the Runner trait.

use walrus_cli::runner::Runner;

/// Verify that Runner is object-safe-enough for static dispatch and that its
/// associated future / stream types satisfy Send.
#[test]
fn runner_trait_bounds() {
    fn assert_runner<R: Runner + Send>() {}
    assert_runner::<walrus_cli::runner::direct::DirectRunner>();
    assert_runner::<walrus_cli::runner::gateway::GatewayRunner>();
}
