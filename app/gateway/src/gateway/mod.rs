//! Protocol impls for the gateway.

use crate::{MemoryBackend, provider::Provider};
use runtime::{DEFAULT_COMPACT_PROMPT, DEFAULT_FLUSH_PROMPT, Hook, Runtime};
use std::sync::Arc;

pub mod builder;
pub mod serve;
pub mod uds;

/// Shared state available to all request handlers.
pub struct Gateway<H: Hook + 'static> {
    /// The walrus runtime (immutable after init).
    pub runtime: Arc<Runtime<H>>,
}

impl<H: Hook + 'static> Clone for Gateway<H> {
    fn clone(&self) -> Self {
        Self {
            runtime: Arc::clone(&self.runtime),
        }
    }
}

/// Type-level hook wiring `MemoryBackend` as the memory implementation.
pub struct GatewayHook;

impl Hook for GatewayHook {
    type Provider = Provider;
    type Memory = MemoryBackend;

    fn compact() -> &'static str {
        DEFAULT_COMPACT_PROMPT
    }

    fn flush() -> &'static str {
        DEFAULT_FLUSH_PROMPT
    }
}
