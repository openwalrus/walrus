//! Transport layer for the Walrus daemon.
//!
//! Wire message types, API traits, and codec live in `walrus-core::protocol`.
//! This crate provides UDS and TCP transport layers.

pub mod tcp;
pub mod uds;
