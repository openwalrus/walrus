//! Provider configuration

use crate::ProviderKind;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// Named provider configuration. Combines identity (`name`) with the fields
/// needed to construct a `Provider`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderConfig {
    /// Unique name for this provider entry. Defaults to `"default"`.
    #[serde(default = "default_name")]
    pub name: CompactString,
    /// Which LLM provider to use.
    #[serde(default)]
    pub provider: ProviderKind,
    /// Model identifier. Not used by `build_provider()` — the caller passes
    /// this to `General::model` when constructing requests.
    pub model: CompactString,
    /// API key (supports `${ENV_VAR}` expansion at the daemon layer).
    #[serde(default)]
    pub api_key: String,
    /// Optional base URL override for the provider endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

fn default_name() -> CompactString {
    CompactString::const_new("default")
}
