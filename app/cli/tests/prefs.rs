//! Tests for config path resolution.

use walrus_cli::config::resolve_config_path;

#[test]
fn config_path_explicit_flag() {
    let path = resolve_config_path(Some("/tmp/my.toml"));
    assert_eq!(path.to_str().unwrap(), "/tmp/my.toml");
}

#[test]
fn config_path_global_default() {
    // With no flag and no workspace config, falls back to global.
    let path = resolve_config_path(None);
    assert!(path.ends_with("walrus/gateway.toml"));
}
