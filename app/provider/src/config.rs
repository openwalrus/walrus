//! Provider configuration
//!
//! Unified config for both remote (API-key-based) and local (model-path-based)
//! providers. Uses `#[serde(tag = "provider", flatten)]` so all fields appear
//! at the same level in TOML (DD#66).

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// Named provider configuration. Combines identity (`name`) with the
/// provider-specific backend settings via `BackendConfig`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderConfig {
    /// Unique name for this provider entry. Defaults to `"default"`.
    #[serde(default = "default_name")]
    pub name: CompactString,
    /// Model identifier. Passed to `General::model` when constructing requests.
    pub model: CompactString,
    /// Provider-specific settings, discriminated by the `provider` field.
    #[serde(flatten)]
    pub backend: BackendConfig,
}

impl ProviderConfig {
    /// Human-readable provider kind string for logging and protocol messages.
    pub fn kind(&self) -> &'static str {
        match &self.backend {
            BackendConfig::DeepSeek(_) => "deepseek",
            BackendConfig::OpenAI(_) => "openai",
            BackendConfig::Grok(_) => "grok",
            BackendConfig::Qwen(_) => "qwen",
            BackendConfig::Kimi(_) => "kimi",
            BackendConfig::Ollama(_) => "ollama",
            BackendConfig::Claude(_) => "claude",
            #[cfg(feature = "local")]
            BackendConfig::Local(_) => "local",
        }
    }
}

/// Provider-specific configuration, discriminated by the `provider` field
/// in TOML/JSON.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum BackendConfig {
    /// DeepSeek API (default).
    DeepSeek(RemoteConfig),
    /// OpenAI API.
    #[serde(rename = "openai")]
    OpenAI(RemoteConfig),
    /// Grok (xAI) API — OpenAI-compatible.
    Grok(RemoteConfig),
    /// Qwen (Alibaba DashScope) API — OpenAI-compatible.
    Qwen(RemoteConfig),
    /// Kimi (Moonshot) API — OpenAI-compatible.
    Kimi(RemoteConfig),
    /// Ollama local API — OpenAI-compatible, no key required.
    Ollama(OllamaConfig),
    /// Claude (Anthropic) Messages API.
    Claude(RemoteConfig),
    /// Local inference via mistralrs (DD#59).
    #[cfg(feature = "local")]
    Local(LocalConfig),
}

/// Configuration for remote HTTP API providers.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RemoteConfig {
    /// API key (supports `${ENV_VAR}` expansion at the daemon layer).
    #[serde(default)]
    pub api_key: String,
    /// Optional base URL override for the provider endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// Configuration for Ollama (OpenAI-compatible, no key required).
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OllamaConfig {
    /// Optional base URL override. Defaults to `http://localhost:11434/v1/chat/completions`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// Configuration for local inference via mistralrs.
#[cfg(feature = "local")]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LocalConfig {
    /// HuggingFace model ID for `TextModelBuilder` (e.g. `"microsoft/Phi-3.5-mini-instruct"`).
    /// Mutually exclusive with `model_path`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Local directory path for `GgufModelBuilder`.
    /// Mutually exclusive with `model_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_path: Option<String>,
    /// GGUF filenames (required when `model_path` is set).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_files: Vec<String>,
    /// In-situ quantization type. `None` means no ISQ.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantization: Option<QuantizationType>,
    /// Optional chat template override (path or inline Jinja).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_template: Option<String>,
}

/// Quantization types supported by mistralrs (maps to `IsqType`).
#[cfg(feature = "local")]
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum QuantizationType {
    /// GGML Q4_0.
    #[serde(rename = "q4_0")]
    Q4_0,
    /// GGML Q4_1.
    #[serde(rename = "q4_1")]
    Q4_1,
    /// GGML Q5_0.
    #[serde(rename = "q5_0")]
    Q5_0,
    /// GGML Q5_1.
    #[serde(rename = "q5_1")]
    Q5_1,
    /// GGML Q8_0.
    #[serde(rename = "q8_0")]
    Q8_0,
    /// GGML Q8_1.
    #[serde(rename = "q8_1")]
    Q8_1,
    /// GGML Q2K.
    #[serde(rename = "q2k")]
    Q2K,
    /// GGML Q3K.
    #[serde(rename = "q3k")]
    Q3K,
    /// GGML Q4K.
    #[serde(rename = "q4k")]
    Q4K,
    /// GGML Q5K.
    #[serde(rename = "q5k")]
    Q5K,
    /// GGML Q6K.
    #[serde(rename = "q6k")]
    Q6K,
    /// GGML Q8K.
    #[serde(rename = "q8k")]
    Q8K,
}

#[cfg(feature = "local")]
impl QuantizationType {
    /// Convert to the mistralrs `IsqType`.
    pub fn to_isq(self) -> mistralrs::IsqType {
        match self {
            Self::Q4_0 => mistralrs::IsqType::Q4_0,
            Self::Q4_1 => mistralrs::IsqType::Q4_1,
            Self::Q5_0 => mistralrs::IsqType::Q5_0,
            Self::Q5_1 => mistralrs::IsqType::Q5_1,
            Self::Q8_0 => mistralrs::IsqType::Q8_0,
            Self::Q8_1 => mistralrs::IsqType::Q8_1,
            Self::Q2K => mistralrs::IsqType::Q2K,
            Self::Q3K => mistralrs::IsqType::Q3K,
            Self::Q4K => mistralrs::IsqType::Q4K,
            Self::Q5K => mistralrs::IsqType::Q5K,
            Self::Q6K => mistralrs::IsqType::Q6K,
            Self::Q8K => mistralrs::IsqType::Q8K,
        }
    }
}

fn default_name() -> CompactString {
    CompactString::const_new("default")
}
