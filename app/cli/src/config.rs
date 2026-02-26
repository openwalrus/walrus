//! Configuration resolution for the CLI.
//!
//! Resolves gateway.toml in priority order:
//! 1. `--config <path>` flag (explicit override)
//! 2. `{cwd}/.walrus/gateway.toml` (workspace config)
//! 3. `~/.config/walrus/gateway.toml` (global default)
//!
//! If the global default doesn't exist, it is generated automatically.

use anyhow::{Context, Result};
use gateway::GatewayConfig;
use std::path::PathBuf;

/// Resolve gateway config following the priority chain.
pub fn resolve_config(config_flag: Option<&str>) -> Result<GatewayConfig> {
    // 1. Explicit --config flag.
    if let Some(path) = config_flag {
        return GatewayConfig::load(path)
            .with_context(|| format!("failed to load config from {path}"));
    }

    // 2. Workspace config: {cwd}/.walrus/gateway.toml
    let workspace_path = PathBuf::from(".walrus/gateway.toml");
    if workspace_path.exists() {
        return GatewayConfig::load(&workspace_path.to_string_lossy())
            .context("failed to load workspace config from .walrus/gateway.toml");
    }

    // 3. Global default: ~/.config/walrus/gateway.toml
    let global_path = global_config_path();
    if global_path.exists() {
        return GatewayConfig::load(&global_path.to_string_lossy())
            .context("failed to load global config");
    }

    // Generate default global config.
    generate_default_config(&global_path)?;
    tracing::info!("generated default config at {}", global_path.display());
    GatewayConfig::load(&global_path.to_string_lossy())
        .context("failed to load generated default config")
}

/// Resolve the config file path (without loading it).
///
/// Same priority chain as [`resolve_config`] but returns just the path.
pub fn resolve_config_path(config_flag: Option<&str>) -> PathBuf {
    if let Some(path) = config_flag {
        return PathBuf::from(path);
    }
    let workspace_path = PathBuf::from(".walrus/gateway.toml");
    if workspace_path.exists() {
        return workspace_path;
    }
    global_config_path()
}

/// Path to the global default config.
fn global_config_path() -> PathBuf {
    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("walrus")
        .join("gateway.toml")
}

/// Generate a default gateway.toml at the given path.
fn generate_default_config(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }
    let contents = toml::to_string_pretty(&GatewayConfig::default())
        .context("failed to serialize default config")?;
    std::fs::write(path, contents)
        .with_context(|| format!("failed to write default config to {}", path.display()))?;
    Ok(())
}
