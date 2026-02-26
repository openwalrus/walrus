//! Gateway hook — type-level runtime configuration for the gateway.

use crate::MemoryBackend;
use deepseek::DeepSeek;
use runtime::{DEFAULT_COMPACT_PROMPT, DEFAULT_FLUSH_PROMPT, Hook};

/// Type-level hook wiring `MemoryBackend` as the memory implementation.
pub struct GatewayHook;

impl Hook for GatewayHook {
    type Provider = DeepSeek;
    type Memory = MemoryBackend;

    fn compact() -> &'static str {
        DEFAULT_COMPACT_PROMPT
    }

    fn flush() -> &'static str {
        DEFAULT_FLUSH_PROMPT
    }
}
