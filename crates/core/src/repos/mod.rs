//! Persistence traits and domain types.
//!
//! [`Storage`] is the unified persistence backend — one trait, one
//! implementation per backend. It replaces the legacy `Repos` composite
//! and four sub-traits which are kept temporarily as a bridge for
//! existing filesystem implementations.

pub mod agents;
#[cfg(feature = "test-utils")]
pub mod mem;
pub mod memory;
pub mod sessions;
pub mod skills;
pub mod storage;

pub use agents::AgentRepo;
pub use memory::{MemoryEntry, MemoryRepo, slugify};
pub use sessions::{SessionHandle, SessionRepo, SessionSnapshot, SessionSummary};
pub use skills::{Skill, SkillRepo};
pub use storage::Storage;

use std::sync::Arc;

/// Composite persistence backend. [`Hook`](crate::Hook) exposes this as
/// a single associated type so `Env` stays at two generic parameters
/// (`H: Host`, `R: Repos`).
pub trait Repos: Send + Sync + 'static {
    type Memory: MemoryRepo;
    type Skills: SkillRepo;
    type Sessions: SessionRepo;
    type Agents: AgentRepo;

    fn memory(&self) -> &Arc<Self::Memory>;
    fn skills(&self) -> &Arc<Self::Skills>;
    fn sessions(&self) -> &Arc<Self::Sessions>;
    fn agents(&self) -> &Arc<Self::Agents>;
}
