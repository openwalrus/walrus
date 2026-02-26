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

/// Default gateway config template generated when no config exists.
const DEFAULT_CONFIG: &str = r#"[server]
host = "127.0.0.1"
port = 3000

[llm]
model = "deepseek-chat"
api_key = "${DEEPSEEK_API_KEY}"

[memory]
backend = "in_memory"

[[agents]]
name = "assistant"
description = "A helpful assistant"
system_prompt = "You are a helpful assistant. Be concise."
"#;

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
    std::fs::write(path, DEFAULT_CONFIG)
        .with_context(|| format!("failed to write default config to {}", path.display()))?;
    Ok(())
}
