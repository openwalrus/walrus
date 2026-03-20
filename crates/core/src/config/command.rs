//! Command service configuration.

use serde::{Deserialize, Serialize};

/// Command service metadata for hub registration.
///
/// Describes a locally-installed command binary. Hub stores this metadata
/// in `crab.toml [commands]` — it does not download or manage the binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    /// Human-readable description.
    pub description: String,
    /// Executable name (resolved via PATH) or absolute path.
    pub binary: String,
    /// Clap subcommand with start/stop/run/logs actions.
    pub subcommand: String,
}
