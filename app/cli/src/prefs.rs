//! CLI preferences stored at `~/.config/walrus/cli.toml`.
//!
//! Separate from `gateway.toml` (runtime config) — this holds CLI-specific
//! preferences like default gateway URL, agent, and model override.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// CLI-specific preferences.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CliPrefs {
    /// Default gateway WebSocket URL.
    pub default_gateway: Option<String>,
    /// Default agent name.
    pub default_agent: Option<String>,
    /// Default model name override.
    pub model: Option<String>,
}

impl CliPrefs {
    /// Load preferences from the default path, returning defaults if missing.
    pub fn load() -> Result<Self> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&contents).with_context(|| format!("parsing {}", path.display()))
    }

    /// Save preferences to the default path.
    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Default path: `~/.config/walrus/cli.toml`.
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("walrus")
            .join("cli.toml")
    }
}
