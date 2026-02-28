//! Configuration resolution for the CLI.
//!
//! Resolves the path to `~/.walrus/walrus.toml` for config commands.

use std::path::PathBuf;

/// Resolve the config file path.
pub fn resolve_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("no home directory")
        .join(".walrus")
        .join("walrus.toml")
}
