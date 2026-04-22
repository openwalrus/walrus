//! Per-agent hook configuration — bash deny rules, memory recall
//! tuning. Each agent owns its own `HooksConfig` directly on
//! [`crate::AgentConfig`]; there is no global override.

use serde::{Deserialize, Serialize};

/// Per-agent hook configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HooksConfig {
    /// Bash tool configuration (`hooks.bash` under an agent).
    pub bash: BashConfig,
    /// Memory hook configuration (`hooks.memory` under an agent).
    pub memory: MemoryConfig,
}

/// Bash tool configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BashConfig {
    /// Disable the bash tool entirely.
    pub disabled: bool,
    /// Reject commands containing any of these strings (e.g. `".ssh"`).
    pub deny: Vec<String>,
}

/// Built-in memory configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    /// Maximum entries returned by auto-recall (default 5).
    pub recall_limit: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self { recall_limit: 5 }
    }
}
