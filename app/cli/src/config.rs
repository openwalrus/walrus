//! Configuration resolution for the CLI.
//!
//! Resolves the path to `~/.config/walrus/gateway.toml` for config commands.

use std::path::PathBuf;

/// Resolve the config file path.
pub fn resolve_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("no platform config directory")
        .join("walrus")
        .join("gateway.toml")
}
