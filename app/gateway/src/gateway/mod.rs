//! Protocol impls for the gateway.

use crate::MemoryBackend;
use crate::{channel::auth::Authenticator, gateway::session::SessionManager};
use deepseek::DeepSeek;
use runtime::{DEFAULT_COMPACT_PROMPT, DEFAULT_FLUSH_PROMPT, Hook, Runtime};
use std::sync::Arc;

pub mod builder;
pub mod serve;
pub mod session;
pub mod uds;

/// Shared state available to all request handlers.
pub struct Gateway<H: Hook + 'static, A: Authenticator> {
    /// The walrus runtime (immutable after init).
    pub runtime: Arc<Runtime<H>>,
    /// Session manager.
    pub sessions: Arc<SessionManager>,
    /// Authenticator.
    pub authenticator: Arc<A>,
}

impl<H: Hook + 'static, A: Authenticator> Clone for Gateway<H, A> {
    fn clone(&self) -> Self {
        Self {
            runtime: Arc::clone(&self.runtime),
            sessions: Arc::clone(&self.sessions),
            authenticator: Arc::clone(&self.authenticator),
        }
    }
}

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
