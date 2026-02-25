//! Walrus agent library.
//!
//! - [`Agent`]: Pure config struct (name, system prompt, tool names).
//! - [`Chat`]: Chat session (agent name + message history).
//! - [`Memory`] / [`InMemory`]: Structured knowledge for system prompts.
//! - [`Embedder`]: Text-to-vector trait for semantic search.
//! - [`Channel`]: Messaging platform integration trait.
//! - [`Skill`] / [`SkillTier`]: Skill data types.

pub use {
    agent::Agent,
    channel::{Attachment, AttachmentKind, Channel, ChannelMessage, Platform},
    chat::Chat,
    embedder::Embedder,
    memory::{InMemory, Memory, MemoryEntry, RecallOptions, with_memory},
    skill::{Skill, SkillTier},
};

mod agent;
mod channel;
mod chat;
mod embedder;
mod memory;
mod skill;
