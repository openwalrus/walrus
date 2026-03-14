//! Service configuration types for `[services.*]` in `walrus.toml`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Kind of managed service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    /// Hook service — speaks WHS protocol over UDS.
    #[default]
    Hook,
    /// Client service — speaks existing walrus protocol (gateways).
    Client,
    /// Arbitrary process — no walrus protocol (e.g. llama-server).
    Process,
}

/// Restart policy for a managed service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Never restart on exit.
    #[default]
    Never,
    /// Restart only on non-zero exit.
    OnFailure,
    /// Always restart.
    Always,
}

/// Per-service configuration from `[services.<name>]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service kind.
    #[serde(default)]
    pub kind: ServiceKind,
    /// Command to execute (binary name or path).
    pub command: String,
    /// Arguments to pass to the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Restart policy.
    #[serde(default)]
    pub restart: RestartPolicy,
    /// Whether the service is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Opaque service-specific configuration (forwarded via WHS Configure).
    #[serde(default)]
    pub config: Value,
}

fn default_true() -> bool {
    true
}
