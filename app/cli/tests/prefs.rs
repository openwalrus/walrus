//! Tests for config path resolution.

use walrus_cli::config::resolve_config_path;

#[test]
fn config_path_global_default() {
    let path = resolve_config_path();
    assert!(path.ends_with(".walrus/gateway.toml"));
}
