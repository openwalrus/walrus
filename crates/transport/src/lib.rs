//! Transport layer for the Crabtalk daemon.
//!
//! Wire message types, API traits, and codec live in `crabtalk-core::protocol`.
//! This crate provides UDS and TCP transport layers.

/// Per-connection reply channel capacity.
///
/// Bounds memory growth when a remote client consumes slowly.
/// At ~50 tokens/sec LLM streaming, 256 messages provides ~5 seconds
/// of buffer before backpressure stalls the producer.
pub const REPLY_CHANNEL_CAPACITY: usize = 256;

pub mod tcp;
#[cfg(unix)]
pub mod uds;
