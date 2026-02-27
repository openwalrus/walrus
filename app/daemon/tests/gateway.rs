//! Gateway integration tests.

use walrus_daemon::GatewayConfig;

/// Verify that GatewayConfig default socket path resolves correctly.
#[test]
fn default_socket_path() {
    let config = GatewayConfig::default();
    let path = config.socket_path(std::path::Path::new("/home/user/.config/walrus"));
    assert_eq!(
        path,
        std::path::PathBuf::from("/home/user/.config/walrus/walrus.sock")
    );
}
