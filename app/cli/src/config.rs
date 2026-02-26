//! Configuration resolution for the CLI.
//!
//! Loads gateway.toml from `~/.config/walrus/gateway.toml`. On first run,
//! scaffolds the config directory with default config and agent files.

use anyhow::{Context, Result};
use gateway::{GatewayConfig, config as gw_config};
use std::path::{Path, PathBuf};

/// Default agent markdown content for first-run scaffold.
const DEFAULT_AGENT_MD: &str = r#"---
name: assistant
description: A helpful assistant
tools:
  - remember
---

You are a helpful assistant. Be concise.
"#;

/// Resolve gateway config from the global config directory.
///
/// If the config directory doesn't exist, scaffolds it with default files.
pub fn resolve_config() -> Result<GatewayConfig> {
    let config_dir = gw_config::global_config_dir();
    let config_path = config_dir.join("gateway.toml");

    if !config_dir.exists() {
        scaffold_config_dir(&config_dir)?;
        tracing::info!("created config directory at {}", config_dir.display());
    }

    GatewayConfig::load(&config_path)
        .with_context(|| format!("failed to load config from {}", config_path.display()))
}

/// Resolve the config file path.
pub fn resolve_config_path() -> PathBuf {
    gw_config::global_config_dir().join("gateway.toml")
}

/// Scaffold the full config directory structure on first run.
fn scaffold_config_dir(config_dir: &Path) -> Result<()> {
    // Create directory structure.
    std::fs::create_dir_all(config_dir.join(gw_config::AGENTS_DIR))
        .context("failed to create agents directory")?;
    std::fs::create_dir_all(config_dir.join(gw_config::SKILLS_DIR))
        .context("failed to create skills directory")?;
    std::fs::create_dir_all(config_dir.join(gw_config::CRON_DIR))
        .context("failed to create cron directory")?;
    std::fs::create_dir_all(config_dir.join(gw_config::DATA_DIR))
        .context("failed to create data directory")?;

    // Write default gateway.toml.
    let gateway_toml = config_dir.join("gateway.toml");
    let contents = toml::to_string_pretty(&GatewayConfig::default())
        .context("failed to serialize default config")?;
    std::fs::write(&gateway_toml, contents)
        .with_context(|| format!("failed to write {}", gateway_toml.display()))?;

    // Write default agent.
    let agent_path = config_dir.join(gw_config::AGENTS_DIR).join("assistant.md");
    std::fs::write(&agent_path, DEFAULT_AGENT_MD)
        .with_context(|| format!("failed to write {}", agent_path.display()))?;

    Ok(())
}
