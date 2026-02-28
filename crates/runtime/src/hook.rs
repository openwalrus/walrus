//! Hook trait — type-level runtime configuration.
//!
//! Hook is a pure trait with no `&self` parameter. It tells the Runtime
//! which model registry and memory backend to use, and what prompts
//! to send for automatic compaction and memory flush.

use wcore::model::{NoopProvider, Registry};
use wcore::{InMemory, Memory};

/// Type-level runtime configuration.
///
/// Determines the model registry, memory backend, and compaction/flush prompts.
/// No instances are created — methods are called as `H::compact()`.
pub trait Hook {
    /// The model registry for this hook (DD#68).
    type Registry: Registry + Send + Sync;

    /// The memory backend for this hook.
    type Memory: Memory;

    /// Compaction prompt sent to the LLM to summarize conversation history
    /// when context approaches the limit. Return empty string to disable.
    fn compact() -> &'static str;

    /// Memory flush prompt sent before compaction to extract durable facts
    /// into memory via the "remember" tool. Return empty string to skip.
    fn flush() -> &'static str;
}

/// Default compaction prompt.
pub const DEFAULT_COMPACT_PROMPT: &str = include_str!("../prompts/compact.md");

/// Default memory flush prompt.
pub const DEFAULT_FLUSH_PROMPT: &str = include_str!("../prompts/flush.md");

impl Hook for () {
    type Registry = NoopProvider;
    type Memory = InMemory;

    fn compact() -> &'static str {
        DEFAULT_COMPACT_PROMPT
    }

    fn flush() -> &'static str {
        DEFAULT_FLUSH_PROMPT
    }
}
