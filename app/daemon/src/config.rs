//! Daemon configuration loaded from TOML (DD#67).

use anyhow::{Context, Result};
use compact_str::CompactString;
pub use model::{ProviderConfig, ProviderManager};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Agents subdirectory (contains *.md files).
pub const AGENTS_DIR: &str = "agents";
/// Skills subdirectory.
pub const SKILLS_DIR: &str = "skills";
/// Cron subdirectory (contains *.md files).
pub const CRON_DIR: &str = "cron";
/// Data subdirectory.
pub const DATA_DIR: &str = "data";
/// SQLite memory database filename.
pub const MEMORY_DB: &str = "memory.db";

/// Resolve the global configuration directory (`~/.walrus/`).
pub fn global_config_dir() -> PathBuf {
    dirs::home_dir().expect("no home directory").join(".walrus")
}

/// Pinned socket path (`~/.walrus/walrus.sock`).
pub fn socket_path() -> PathBuf {
    global_config_dir().join("walrus.sock")
}

/// Top-level daemon configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// LLM provider configurations (`[[models]]` array).
    pub models: Vec<ProviderConfig>,
    /// Channel configurations.
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
    /// MCP server configurations.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            models: vec![ProviderConfig {
                model: "deepseek-chat".into(),
                api_key: Some("${DEEPSEEK_API_KEY}".to_owned()),
                base_url: None,
                loader: None,
                quantization: None,
                chat_template: None,
            }],
            channels: Vec::new(),
            mcp_servers: Vec::new(),
        }
    }
}

/// Channel configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Platform name.
    pub platform: CompactString,
    /// Bot token (supports `${ENV_VAR}` expansion).
    pub bot_token: String,
    /// Default agent for this channel.
    pub agent: CompactString,
    /// Optional specific channel ID for exact routing.
    pub channel_id: Option<CompactString>,
}

/// MCP server configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name.
    pub name: CompactString,
    /// Command to spawn.
    pub command: String,
    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables.
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    /// Auto-restart on failure.
    #[serde(default = "default_true")]
    pub auto_restart: bool,
}

fn default_true() -> bool {
    true
}

/// Default agent markdown content for first-run scaffold.
pub const DEFAULT_AGENT_MD: &str = r#"---
name: assistant
description: A helpful assistant
tools:
  - remember
---

You are a helpful assistant. Be concise.
"#;

impl DaemonConfig {
    /// Parse a TOML string into a `DaemonConfig`, expanding environment
    /// variables in supported fields.
    pub fn from_toml(toml_str: &str) -> anyhow::Result<Self> {
        let expanded = crate::utils::expand_env_vars(toml_str);
        let config: Self = toml::from_str(&expanded)?;
        Ok(config)
    }

    /// Load configuration from a file path.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content)
    }
}

/// Scaffold the full config directory structure on first run.
///
/// Creates subdirectories (agents, skills, cron, data), writes a default
/// walrus.toml and a default assistant agent markdown file.
pub fn scaffold_config_dir(config_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(config_dir.join(AGENTS_DIR))
        .context("failed to create agents directory")?;
    std::fs::create_dir_all(config_dir.join(SKILLS_DIR))
        .context("failed to create skills directory")?;
    std::fs::create_dir_all(config_dir.join(CRON_DIR))
        .context("failed to create cron directory")?;
    std::fs::create_dir_all(config_dir.join(DATA_DIR))
        .context("failed to create data directory")?;

    let gateway_toml = config_dir.join("walrus.toml");
    let contents = toml::to_string_pretty(&DaemonConfig::default())
        .context("failed to serialize default config")?;
    std::fs::write(&gateway_toml, contents)
        .with_context(|| format!("failed to write {}", gateway_toml.display()))?;

    let agent_path = config_dir.join(AGENTS_DIR).join("assistant.md");
    std::fs::write(&agent_path, DEFAULT_AGENT_MD)
        .with_context(|| format!("failed to write {}", agent_path.display()))?;

    Ok(())
}
