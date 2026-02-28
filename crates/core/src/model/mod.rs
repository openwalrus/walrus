//! Unified LLM interface types and traits.
//!
//! Provides the shared types used across all LLM providers:
//! `Message`, `Response`, `StreamChunk`, `Tool`, `General`, and the `LLM` trait.

pub use config::General;
pub use limits::default_context_limit;
pub use message::{Message, MessageBuilder, Role, estimate_tokens};
pub use noop::NoopProvider;
pub use provider::{LLM, Registry};
pub use response::{
    Choice, CompletionMeta, CompletionTokensDetails, Delta, FinishReason, Response, Usage,
};
pub use stream::StreamChunk;
pub use tool::{FunctionCall, Tool, ToolCall, ToolChoice};

mod config;
mod limits;
mod message;
mod noop;
mod provider;
mod response;
mod stream;
mod tool;
