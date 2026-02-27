//! Gateway configuration loaded from TOML.

use anyhow::{Context, Result};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Config directory name under platform config dir.
pub const CONFIG_DIR: &str = "walrus";
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

/// Resolve the global configuration directory (`~/.config/walrus/` on unix).
pub fn global_config_dir() -> std::path::PathBuf {
    dirs::config_dir()
        .expect("no platform config directory")
        .join(CONFIG_DIR)
}

/// Top-level gateway configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Server bind configuration.
    pub server: ServerConfig,
    /// LLM provider configuration.
    pub llm: LlmConfig,
    /// Memory backend configuration.
    #[serde(default)]
    pub memory: MemoryConfig,
    /// Channel configurations.
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
    /// MCP server configurations.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}

/// Server configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Custom Unix domain socket path. When `None`, defaults to
    /// `<config_dir>/walrus.sock`.
    pub socket_path: Option<String>,
}

/// LLM provider configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Which LLM provider to use.
    #[serde(default)]
    pub provider: ProviderKind,
    /// Model identifier.
    pub model: CompactString,
    /// API key (supports `${ENV_VAR}` expansion).
    #[serde(default)]
    pub api_key: String,
    /// Optional base URL override for the provider endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: ProviderKind::DeepSeek,
            model: "deepseek-chat".into(),
            api_key: "${DEEPSEEK_API_KEY}".to_owned(),
            base_url: None,
        }
    }
}

/// Supported LLM provider kinds.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// DeepSeek API (default).
    #[default]
    DeepSeek,
    /// OpenAI API.
    OpenAI,
    /// Grok (xAI) API — OpenAI-compatible.
    Grok,
    /// Qwen (Alibaba DashScope) API — OpenAI-compatible.
    Qwen,
    /// Kimi (Moonshot) API — OpenAI-compatible.
    Kimi,
    /// Ollama local API — OpenAI-compatible, no key required.
    Ollama,
    /// Claude (Anthropic) Messages API.
    Claude,
}

/// Memory backend configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    /// Backend type: "in_memory" or "sqlite".
    pub backend: MemoryBackendKind,
}

/// Memory backend kind.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryBackendKind {
    /// In-memory backend (no persistence).
    #[default]
    InMemory,
    /// SQLite-backed persistent memory.
    Sqlite,
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

impl GatewayConfig {
    /// Parse a TOML string into a `GatewayConfig`, expanding environment
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

    /// Resolve the socket path. Uses the explicit config value if set,
    /// otherwise defaults to `<config_dir>/walrus.sock`.
    pub fn socket_path(&self, config_dir: &std::path::Path) -> std::path::PathBuf {
        self.server
            .socket_path
            .as_ref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| config_dir.join("walrus.sock"))
    }
}

/// Scaffold the full config directory structure on first run.
///
/// Creates subdirectories (agents, skills, cron, data), writes a default
/// gateway.toml and a default assistant agent markdown file.
pub fn scaffold_config_dir(config_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(config_dir.join(AGENTS_DIR))
        .context("failed to create agents directory")?;
    std::fs::create_dir_all(config_dir.join(SKILLS_DIR))
        .context("failed to create skills directory")?;
    std::fs::create_dir_all(config_dir.join(CRON_DIR))
        .context("failed to create cron directory")?;
    std::fs::create_dir_all(config_dir.join(DATA_DIR))
        .context("failed to create data directory")?;

    let gateway_toml = config_dir.join("gateway.toml");
    let contents = toml::to_string_pretty(&GatewayConfig::default())
        .context("failed to serialize default config")?;
    std::fs::write(&gateway_toml, contents)
        .with_context(|| format!("failed to write {}", gateway_toml.display()))?;

    let agent_path = config_dir.join(AGENTS_DIR).join("assistant.md");
    std::fs::write(&agent_path, DEFAULT_AGENT_MD)
        .with_context(|| format!("failed to write {}", agent_path.display()))?;

    Ok(())
}
