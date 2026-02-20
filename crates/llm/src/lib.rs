//! Unified LLM interface types and traits.
//!
//! This crate provides the shared types used across all LLM providers:
//! `Message`, `Response`, `StreamChunk`, `Tool`, `Config`, and the `LLM` trait.

pub use {
    config::{Config, General},
    message::{Message, Role},
    provider::LLM,
    reqwest::{self, Client},
    response::{FinishReason, Response, Usage},
    stream::StreamChunk,
    tool::{FunctionCall, Tool, ToolCall, ToolChoice},
};

mod config;
mod message;
mod provider;
mod response;
mod stream;
mod tool;
