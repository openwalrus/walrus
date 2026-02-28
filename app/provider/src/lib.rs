//! Provider crate — centralizes LLM provider enum dispatch, configuration,
//! construction, and runtime management.
//!
//! `Provider` enum wraps concrete backends (DeepSeek, OpenAI, Claude, Local)
//! behind a unified `LLM` impl. `ProviderManager` holds a named map of
//! providers with concurrent-safe active-provider swapping (DD#60, DD#65).
//! Config uses `BackendConfig` tagged enum to describe both remote and local
//! providers in a single type (DD#66).

pub mod config;
pub mod manager;
mod provider;

pub use {
    config::{BackendConfig, ProviderConfig},
    manager::ProviderManager,
    provider::{Provider, build_provider},
};
