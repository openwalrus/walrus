//! Memory subsystem configuration.

use serde::{Deserialize, Serialize};

/// Memory subsystem configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    /// Enable automatic memory recall before each agent run (default: true).
    #[serde(default = "default_true")]
    pub auto_recall: bool,
}

fn default_true() -> bool {
    true
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self { auto_recall: true }
    }
}
