//! Model crate — LLM provider implementations, enum dispatch, configuration,
//! construction, and runtime management.
//!
//! Merges all provider backends (DeepSeek, OpenAI, Claude, Local) with the
//! `Provider` enum, `ProviderManager`, and `ProviderConfig` into a single crate.
//! Config uses flat `ProviderConfig` with model-prefix kind detection (DD#67).

pub mod config;
pub mod http;
pub mod manager;
mod provider;
mod request;

pub mod claude;
pub mod deepseek;
#[cfg(feature = "local")]
pub mod local;
pub mod openai;

pub use config::{ProviderConfig, ProviderKind};
pub use http::HttpProvider;
pub use manager::ProviderManager;
pub use provider::{Provider, build_provider};
pub use reqwest::Client;
