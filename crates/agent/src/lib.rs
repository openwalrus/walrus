//! Cydonia agent library.
//!
//! - [`Agent`]: Pure config struct (name, system prompt, tool names).
//! - [`Memory`] / [`InMemory`]: Structured knowledge for system prompts.

pub use {
    agent::Agent,
    memory::{InMemory, Memory, with_memory},
};

mod agent;
mod memory;
