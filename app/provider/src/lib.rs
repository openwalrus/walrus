//! Provider crate — centralizes LLM provider enum dispatch, configuration,
//! construction, and runtime management.
//!
//! `Provider` enum wraps concrete backends (DeepSeek, OpenAI, Claude, Mistral)
//! behind a unified `LLM` impl. `ProviderManager` holds a named map of
//! providers with concurrent-safe active-provider swapping (DD#60, DD#65).

mod config;
pub mod manager;
mod provider;

pub use {
    config::ProviderConfig,
    manager::ProviderManager,
    provider::{Provider, ProviderKind, build_provider},
};
