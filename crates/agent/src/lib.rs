//! Cydonia agent framework.
//!
//! - [`Agent`]: Pure config struct (name, system prompt, tool names).
//! - [`Provider`]: Static-dispatch enum over LLM implementations.
//! - [`Chat`]: Orchestrates agent + provider + runtime.
//! - [`Memory`] / [`InMemory`]: Structured knowledge for system prompts.
//! - [`build_team`]: Compose agents into teams via the runtime.

pub use {
    agent::Agent,
    chat::Chat,
    memory::{InMemory, Memory, with_memory},
    provider::Provider,
    team::{build_team, extract_input, worker_tool},
};

mod agent;
mod chat;
mod memory;
mod provider;
pub mod team;
