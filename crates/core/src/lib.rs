//! Cydonia core library
//!
//! This library gathers the user interfaces of cydonia

pub mod chat;
pub mod config;
pub mod manifest;

pub use {chat::Message, manifest::Manifest};
