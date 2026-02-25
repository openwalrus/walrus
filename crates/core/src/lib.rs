//! Walrus agent library.
//!
//! - [`Agent`]: Pure config struct (name, system prompt, tool names).
//! - [`Chat`]: Chat session (agent name + message history).
//! - [`Memory`] / [`InMemory`]: Structured knowledge for system prompts.
//! - [`Embedder`]: Text-to-vector trait for semantic search.
//! - [`Channel`]: Messaging platform integration trait.
//! - [`Skill`] / [`SkillTier`]: Skill data types.

pub use agent::Agent;
pub use channel::{Attachment, AttachmentKind, Channel, ChannelMessage, Platform};
pub use chat::Chat;
pub use embedder::Embedder;
pub use memory::{InMemory, Memory, MemoryEntry, RecallOptions, with_memory};
pub use skill::{Skill, SkillTier};

mod agent;
mod channel;
mod chat;
mod embedder;
mod memory;
mod skill;
