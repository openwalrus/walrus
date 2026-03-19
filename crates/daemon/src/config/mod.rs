//! Daemon configuration loaded from TOML.

pub use crate::hook::{
    mcp::McpServerConfig,
    os::{PermissionConfig, ToolPermission},
    system::SystemConfig,
};
pub use ::model::{ModelConfig, ProviderDef, ProviderRegistry};
use anyhow::Result;
pub use loader::{load_agents_dir, scaffold_config_dir};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use wcore::{
    AgentConfig, HeartbeatConfig,
    paths::{AGENTS_DIR, CONFIG_DIR, DATA_DIR, HOME_DIR, MEMORY_DB, SKILLS_DIR, SOCKET_PATH},
};

mod loader;

/// Top-level daemon configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DaemonConfig {
    /// Provider definitions (`[provider.<name>]`).
    #[serde(default)]
    pub provider: BTreeMap<String, ProviderDef>,
    /// Model configuration (embedding model).
    #[serde(default)]
    pub model: ModelConfig,
    /// MCP server configurations.
    #[serde(default)]
    pub mcps: BTreeMap<String, McpServerConfig>,
    /// System configuration (tasks + memory).
    #[serde(default)]
    pub system: SystemConfig,
    /// Per-agent configurations (name → config).
    #[serde(default)]
    pub agents: BTreeMap<String, AgentConfig>,
    /// Permission configuration: global defaults + per-agent overrides.
    #[serde(default)]
    pub permissions: PermissionConfig,
    /// Managed child services (`[services.<name>]`).
    #[serde(default)]
    pub services: BTreeMap<String, crate::service::ServiceConfig>,
}

impl DaemonConfig {
    /// Parse a TOML string into a `DaemonConfig`.
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        let mut config: Self = toml::from_str(toml_str)?;
        config.mcps.iter_mut().for_each(|(name, server)| {
            if server.name.is_empty() {
                server.name = name.clone();
            }
        });
        if config.system.crab.model.is_none() {
            config.system.crab.model = Some(::model::default_model().into());
        }
        ModelConfig::validate(&config.provider)?;
        Ok(config)
    }

    /// Load configuration from a file path.
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content)
    }
}
