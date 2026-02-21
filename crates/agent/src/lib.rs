//! Cydonia agent library.
//!
//! - [`Agent`]: Pure config struct (name, system prompt, tool names).
//! - [`Chat`]: Chat session (agent name + message history).
//! - [`Memory`] / [`InMemory`]: Structured knowledge for system prompts.

pub use {
    agent::Agent,
    chat::Chat,
    memory::{InMemory, Memory, with_memory},
};

mod agent;
mod chat;
mod memory;
