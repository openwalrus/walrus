//! Cydonia core library
//!
//! This library gathers the user interfaces of cydonia

pub mod chat;
pub mod manifest;

pub use {
    chat::Message,
    manifest::{Family, Quantization, Release},
};

/// The tokenizer repo of cydonia in huggingface.
pub const TOKENIZER: &str = "clearloop/tokenizer";
