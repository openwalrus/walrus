//! Walrus agent library.
//!
//! - [`Agent`]: Pure config struct (name, system prompt, tool names).
//! - [`Memory`] / [`InMemory`]: Structured knowledge for system prompts.
//! - [`Embedder`]: Text-to-vector trait for semantic search.
//! - [`Channel`]: Messaging platform integration trait.
//! - [`Skill`] / [`SkillTier`]: Skill data types.
//! - [`model`]: Unified LLM interface types and traits.

pub use agent::Agent;
pub use channel::{Attachment, AttachmentKind, Channel, ChannelMessage, Platform};
pub use memory::{Embedder, InMemory, Memory, MemoryEntry, NoEmbedder, RecallOptions, with_memory};
pub use skill::{Skill, SkillTier};

mod agent;
mod channel;
pub mod memory;
pub mod model;
mod skill;
