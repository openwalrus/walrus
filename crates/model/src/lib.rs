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

pub use config::{ApiStandard, ProviderDef, validate_providers};
pub use manager::ProviderRegistry;
pub use provider::{Provider, build_provider};
pub use reqwest::Client;
