//! Daemon configuration loaded from `config.toml`.

use crate::{ProviderDef, config::system::TasksConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level daemon configuration (`config.toml`).
///
/// Holds immutable per-install settings: providers, task executor pool,
/// and env vars passed to MCP processes. Mutable runtime records (MCPs,
/// agents) live in [`crate::storage::Storage`]. Per-agent customization
/// (hooks, etc.) lives directly on each [`crate::AgentConfig`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DaemonConfig {
    /// Provider definitions (`[provider.<name>]`).
    #[serde(default)]
    pub provider: BTreeMap<String, ProviderDef>,
    /// Task executor pool configuration (`[tasks]`).
    #[serde(default)]
    pub tasks: TasksConfig,
    /// Environment variables passed to all MCP server processes.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl DaemonConfig {
    /// Parse a TOML string into a `DaemonConfig`.
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        let config: Self = toml::from_str(toml_str)?;
        validate_providers(&config.provider)?;
        Ok(config)
    }

    /// Load configuration from a file path.
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content)
    }
}

/// Validate provider definitions and reject duplicate model names.
pub fn validate_providers(providers: &BTreeMap<String, ProviderDef>) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for (name, def) in providers {
        def.validate(name).map_err(|e| anyhow::anyhow!(e))?;
        for model in &def.models {
            if !seen.insert(model.clone()) {
                anyhow::bail!("duplicate model name '{model}' across providers");
            }
        }
    }
    Ok(())
}
