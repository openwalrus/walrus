//! Model crate — LLM provider implementations via crabtalk, enum dispatch,
//! configuration, construction, and runtime management.
//!
//! Uses `crabtalk-provider` for the actual LLM backends (OpenAI, Anthropic,
//! Google, Bedrock, Azure). Wraps them behind wcore's `Model` trait with
//! type conversion and retry logic.

pub mod config;
mod convert;
pub mod manager;
mod provider;

/// Default model name when none is configured.
pub fn default_model() -> &'static str {
    "deepseek-chat"
}

pub use config::{ApiStandard, ModelConfig, ProviderDef};
pub use manager::ProviderRegistry;
pub use provider::{Provider, build_provider};
pub use reqwest::Client;
