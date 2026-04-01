use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-model token pricing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    /// Cost per million prompt tokens in USD.
    pub prompt_cost_per_million: f64,
    /// Cost per million completion tokens in USD.
    pub completion_cost_per_million: f64,
}

/// Compute the cost in USD for a given number of prompt and completion tokens.
pub fn cost(pricing: &PricingConfig, prompt_tokens: u32, completion_tokens: u32) -> f64 {
    (prompt_tokens as f64 * pricing.prompt_cost_per_million
        + completion_tokens as f64 * pricing.completion_cost_per_million)
        / 1_000_000.0
}

/// Top-level gateway configuration, loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Address to listen on, e.g. "0.0.0.0:8080".
    pub listen: String,
    /// Named provider configurations.
    pub providers: HashMap<String, ProviderConfig>,
    /// Virtual API keys for client authentication.
    #[serde(default)]
    pub keys: Vec<KeyConfig>,
    /// Extension configurations. Each key is an extension name, value is its config.
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
    /// Storage backend configuration.
    #[serde(default)]
    pub storage: Option<StorageConfig>,
    /// Model name aliases. Maps friendly names to canonical model names.
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    /// Per-model token pricing for cost tracking and budget enforcement.
    #[serde(default)]
    pub pricing: HashMap<String, PricingConfig>,
    /// Admin API bearer token. If set, enables /v1/admin/* endpoints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_token: Option<String>,
    /// Graceful shutdown timeout in seconds. Default: 30.
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout: u64,
}

/// Configuration for a single LLM provider.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider kind determines the dispatch path.
    #[serde(default, skip_serializing_if = "ProviderKind::is_default")]
    pub kind: ProviderKind,
    /// API key (supports `${ENV_VAR}` interpolation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Base URL override. OpenAI-compat providers have sensible defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Model names served by this provider.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
    /// Routing weight for weighted random selection. Higher = more traffic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<u16>,
    /// Max retries on transient errors before fallback. 0 disables retry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    /// API version string, used by Azure OpenAI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// Per-request timeout in seconds. Default: 30.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// AWS region for Bedrock provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// AWS access key ID for Bedrock provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_key: Option<String>,
    /// AWS secret access key for Bedrock provider.
    #[serde(default, skip_serializing)]
    pub secret_key: Option<String>,
    /// Path to a GGUF model file for the LlamaCpp provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_path: Option<String>,
    /// Number of GPU layers to offload (LlamaCpp). Default: 0 (CPU only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_gpu_layers: Option<u32>,
    /// Context size in tokens (LlamaCpp). Default: 2048.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_ctx: Option<u32>,
    /// Number of threads for inference (LlamaCpp). Default: system-chosen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_threads: Option<u32>,
}

fn default_shutdown_timeout() -> u64 {
    30
}

/// Which provider implementation to use.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    #[serde(alias = "openai_compat")]
    Openai,
    Anthropic,
    Google,
    Bedrock,
    Ollama,
    Azure,
    #[serde(alias = "llama_cpp")]
    LlamaCpp,
}

impl ProviderKind {
    /// Returns true if this is the default variant (Openai).
    pub fn is_default(&self) -> bool {
        *self == Self::Openai
    }
}

impl ProviderConfig {
    /// Resolve the effective provider kind.
    ///
    /// Returns `Anthropic` if the field is explicitly set to `Anthropic`,
    /// or if `base_url` contains "anthropic". Otherwise returns the
    /// configured kind.
    pub fn effective_kind(&self) -> ProviderKind {
        if self.kind == ProviderKind::Anthropic {
            return ProviderKind::Anthropic;
        }
        if let Some(url) = &self.base_url
            && url.contains("anthropic")
        {
            return ProviderKind::Anthropic;
        }
        self.kind
    }

    /// Validate field combinations.
    pub fn validate(&self, provider_name: &str) -> Result<(), String> {
        if self.models.is_empty() {
            return Err(format!("provider '{provider_name}' has no models"));
        }
        match self.kind {
            ProviderKind::Bedrock => {
                if self.region.is_none() {
                    return Err(format!(
                        "provider '{provider_name}' (bedrock) requires region"
                    ));
                }
                if self.access_key.is_none() {
                    return Err(format!(
                        "provider '{provider_name}' (bedrock) requires access_key"
                    ));
                }
                if self.secret_key.is_none() {
                    return Err(format!(
                        "provider '{provider_name}' (bedrock) requires secret_key"
                    ));
                }
            }
            ProviderKind::Ollama => {
                // Ollama doesn't require api_key or base_url.
            }
            ProviderKind::LlamaCpp => match &self.model_path {
                None => {
                    return Err(format!(
                        "provider '{provider_name}' (llamacpp) requires model_path"
                    ));
                }
                Some(path) => {
                    if !std::path::Path::new(path).exists() {
                        return Err(format!(
                            "provider '{provider_name}' (llamacpp): model_path '{path}' does not exist"
                        ));
                    }
                }
            },
            _ => {
                if self.api_key.is_none() && self.base_url.is_none() {
                    return Err(format!(
                        "provider '{provider_name}' requires api_key or base_url"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Virtual API key for client authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyConfig {
    /// Human-readable name for this key.
    pub name: String,
    /// The key string clients send in Authorization header.
    pub key: String,
    /// Which models this key can access. `["*"]` means all.
    pub models: Vec<String>,
}

/// Storage backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Backend kind: "memory" (default) or "sqlite" (requires feature).
    #[serde(default = "StorageConfig::default_kind")]
    pub kind: String,
    /// File path for persistent backends (required for sqlite).
    #[serde(default)]
    pub path: Option<String>,
}

impl StorageConfig {
    fn default_kind() -> String {
        "memory".to_string()
    }
}

impl GatewayConfig {
    /// Load config from a TOML file, expanding `${VAR}` patterns in string values.
    #[cfg(feature = "gateway")]
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let raw = std::fs::read_to_string(path)?;
        let expanded = expand_env_vars(&raw);
        let config: GatewayConfig = toml::from_str(&expanded)?;
        Ok(config)
    }
}

/// Expand `${VAR}` patterns in a string using environment variables.
/// Unknown variables are replaced with empty string.
#[cfg(feature = "gateway")]
fn expand_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut var_name = String::new();
            for ch in chars.by_ref() {
                if ch == '}' {
                    break;
                }
                var_name.push(ch);
            }
            if let Ok(val) = std::env::var(&var_name) {
                result.push_str(&val);
            }
        } else {
            result.push(c);
        }
    }

    result
}
