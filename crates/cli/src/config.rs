//! Configuration for the CLI

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::LazyLock};

static CONFIG: LazyLock<PathBuf> =
    LazyLock::new(|| dirs::home_dir().unwrap().join(".config/ullm.toml"));

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// The configuration for the CLI
    pub config: cydonia::General,

    /// The API keys for LLMs
    pub key: BTreeMap<String, String>,
}

impl Config {
    /// Load the configuration from the file
    pub fn load() -> Result<Self> {
        let config = toml::from_str(&std::fs::read_to_string(CONFIG.as_path())?)?;
        Ok(config)
    }

    /// Save the configuration to the file
    pub fn save(&self) -> Result<()> {
        std::fs::write(CONFIG.as_path(), toml::to_string(self)?)?;
        tracing::info!("Configuration saved to {}", CONFIG.display());
        Ok(())
    }

    /// Get the core config
    pub fn config(&self) -> &cydonia::General {
        &self.config
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config: cydonia::General::default(),
            key: [("deepseek".to_string(), "YOUR_API_KEY".to_string())]
                .into_iter()
                .collect::<_>(),
        }
    }
}
