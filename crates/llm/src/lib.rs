//! Unified LLM interface types and traits.
//!
//! This crate provides the shared types used across all LLM providers:
//! `Message`, `Response`, `StreamChunk`, `Tool`, `Config`, and the `LLM` trait.
//! Also provides `HttpProvider` for OpenAI-compatible HTTP transport (DD#58)
//! and a shared `Request` type.

pub use config::{Config, General};
#[cfg(feature = "http")]
pub use http::HttpProvider;
pub use message::{Message, Role, estimate_tokens};
pub use noop::NoopProvider;
pub use provider::LLM;
#[cfg(feature = "http")]
pub use request::Request;
#[cfg(feature = "http")]
pub use reqwest::{self, Client};
pub use response::{
    Choice, CompletionMeta, CompletionTokensDetails, Delta, FinishReason, Response, Usage,
};
pub use stream::StreamChunk;
pub use tool::{FunctionCall, Tool, ToolCall, ToolChoice};

mod config;
#[cfg(feature = "http")]
mod http;
mod message;
mod noop;
mod provider;
#[cfg(feature = "http")]
mod request;
mod response;
mod stream;
mod tool;
