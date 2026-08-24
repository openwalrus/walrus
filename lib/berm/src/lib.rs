//! Reach the Crabtalk runtime from a harness.
//!
//! [`berm-lang`](https://crates.io/crates/berm-lang) is the sandbox: it knows
//! how to be a harness and owns the ABI, and it serves no system harness of its
//! own — what a harness can reach is whatever its host registered.
//!
//! This crate is the guest half of everything Crabtalk registers: files,
//! commands, HTTP, and the runtime itself, all under the `crabtalk` namespace.
//!
//! ```ignore
//! use berm_crabtalk::protocol;
//!
//! let agents = protocol::peers()?;
//! ```
//!
//! One door per operation, so a caller matches on nothing: what it asked for is
//! what it gets back, or an error.

#![no_std]

extern crate alloc;

pub mod protocol;
pub mod sys;

pub use proto;
pub use sys::{exec, fs, http};
