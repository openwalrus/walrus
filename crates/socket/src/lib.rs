//! Unix domain socket transport for the Walrus daemon.
//!
//! Wire message types, API traits, and codec live in `walrus-core::protocol`.
//! This crate provides only the UDS transport layer.

pub mod client;
pub mod server;

pub use client::{ClientConfig, Connection, WalrusClient};
