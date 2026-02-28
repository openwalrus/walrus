//! Provider configuration (DD#67).
//!
//! Flat `ProviderConfig` with optional fields for both remote and local
//! providers. Provider kind inferred from model name prefix via `kind()`.
//! `Loader` selects which mistralrs builder to use for local models.

use anyhow::{Result, bail};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// Flat provider configuration. All fields except `model` are optional.
/// Provider kind is inferred from the model name — no explicit `provider` tag.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderConfig {
    /// Model identifier. Remote models use known prefixes (`deepseek-*`,
    /// `gpt-*`, `claude-*`, etc.). Local models use HuggingFace repo IDs
    /// containing `/` (e.g. `microsoft/Phi-3.5-mini-instruct`).
    pub model: CompactString,
    /// API key for remote providers. Supports `${ENV_VAR}` expansion at the
    /// daemon layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Base URL override for remote providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Mistralrs model builder to use for local models. Defaults to `Text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader: Option<Loader>,
    /// In-situ quantization for local models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantization: Option<QuantizationType>,
    /// Chat template override for local models (path or inline Jinja).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_template: Option<String>,
}

impl ProviderConfig {
    /// Detect the provider kind from the model name.
    pub fn kind(&self) -> Result<ProviderKind> {
        ProviderKind::from_model(&self.model)
    }

    /// Validate field combinations.
    ///
    /// Called on startup and on provider add/reload.
    pub fn validate(&self) -> Result<()> {
        if self.model.is_empty() {
            bail!("model is required");
        }

        let kind = self.kind()?;

        match kind {
            ProviderKind::Local => {
                if self.api_key.is_some() {
                    bail!("local provider '{}' must not have api_key", self.model);
                }
            }
            _ => {
                // Remote providers: api_key is required unless base_url is set
                // (e.g. Ollama which is keyless with a local base_url).
                if self.api_key.is_none() && self.base_url.is_none() {
                    bail!(
                        "remote provider '{}' requires api_key or base_url",
                        self.model
                    );
                }
                if self.loader.is_some() {
                    bail!(
                        "remote provider '{}' must not have loader field",
                        self.model
                    );
                }
                if self.quantization.is_some() {
                    bail!(
                        "remote provider '{}' must not have quantization field",
                        self.model
                    );
                }
                if self.chat_template.is_some() {
                    bail!(
                        "remote provider '{}' must not have chat_template field",
                        self.model
                    );
                }
            }
        }

        Ok(())
    }
}

/// Provider kind, inferred from the model name at runtime.
///
/// Not serialized — purely a dispatch enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    DeepSeek,
    OpenAI,
    Claude,
    Grok,
    Qwen,
    Kimi,
    Local,
}

impl ProviderKind {
    /// Detect provider kind from a model name string.
    ///
    /// Rules:
    /// 1. If model contains `/` → Local (HuggingFace repo ID).
    /// 2. Otherwise, match known remote prefixes.
    /// 3. No match → error.
    pub fn from_model(model: &str) -> Result<Self> {
        if model.contains('/') {
            return Ok(Self::Local);
        }

        let prefixes: &[(&[&str], ProviderKind)] = &[
            (&["deepseek-"], ProviderKind::DeepSeek),
            (&["gpt-", "o1-", "o3-", "o4-"], ProviderKind::OpenAI),
            (&["claude-"], ProviderKind::Claude),
            (&["grok-"], ProviderKind::Grok),
            (&["qwen-", "qwq-"], ProviderKind::Qwen),
            (&["kimi-", "moonshot-"], ProviderKind::Kimi),
        ];

        for (patterns, kind) in prefixes {
            for prefix in *patterns {
                if model.starts_with(prefix) {
                    return Ok(*kind);
                }
            }
        }

        bail!("unknown model prefix: '{model}' — cannot detect provider kind")
    }

    /// Human-readable name for logging.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeepSeek => "deepseek",
            Self::OpenAI => "openai",
            Self::Claude => "claude",
            Self::Grok => "grok",
            Self::Qwen => "qwen",
            Self::Kimi => "kimi",
            Self::Local => "local",
        }
    }
}

/// Selects which mistralrs model builder to use for local inference.
///
/// Defaults to `Text` when omitted in config.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum Loader {
    /// `TextModelBuilder` — standard text models.
    #[default]
    Text,
    /// `LoraModelBuilder` — LoRA adapter models.
    Lora,
    /// `XLoraModelBuilder` — X-LoRA adapter models.
    #[serde(rename = "xlora")]
    XLora,
    /// `GgufModelBuilder` — GGUF quantized models.
    Gguf,
    /// `GgufLoraModelBuilder` — GGUF + LoRA.
    #[serde(rename = "gguf_lora")]
    GgufLora,
    /// `GgufXLoraModelBuilder` — GGUF + X-LoRA.
    #[serde(rename = "gguf_xlora")]
    GgufXLora,
    /// `VisionModelBuilder` — vision-language models.
    Vision,
}

/// Quantization types supported by mistralrs (maps to `IsqType`).
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum QuantizationType {
    #[serde(rename = "q4_0")]
    Q4_0,
    #[serde(rename = "q4_1")]
    Q4_1,
    #[serde(rename = "q5_0")]
    Q5_0,
    #[serde(rename = "q5_1")]
    Q5_1,
    #[serde(rename = "q8_0")]
    Q8_0,
    #[serde(rename = "q8_1")]
    Q8_1,
    #[serde(rename = "q2k")]
    Q2K,
    #[serde(rename = "q3k")]
    Q3K,
    #[serde(rename = "q4k")]
    Q4K,
    #[serde(rename = "q5k")]
    Q5K,
    #[serde(rename = "q6k")]
    Q6K,
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
