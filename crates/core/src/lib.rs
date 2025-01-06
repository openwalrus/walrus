//! Cydonia core library
//!
//! This library gathers the user interfaces of cydonia

mod chat;
mod manifest;

pub use {chat::Message, manifest::Manifest};

/// The tokenizer repo of cydonia in huggingface.
pub const TOKENIZER: &str = "clearloop/tokenizer";
