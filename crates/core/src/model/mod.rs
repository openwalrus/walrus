//! Unified LLM interface types and the `Model<P>` wrapper.
//!
//! Provides the wcore-typed types used by `Agent` and `Runtime`:
//! `Message`, `Request`, `Response`, `StreamChunk`, `Tool`, plus the
//! `Model<P>` newtype that wraps any `crabllm_core::Provider` and exposes
//! `send`/`stream` over wcore types.

pub use client::Model;
pub use limits::default_context_limit;
pub use message::{Message, MessageBuilder, Role, estimate_tokens};
pub use request::Request;
pub use response::{
    Choice, CompletionMeta, CompletionTokensDetails, Delta, FinishReason, Response, Usage,
};
pub use stream::StreamChunk;
pub use tool::{FunctionCall, Tool, ToolCall, ToolChoice};

mod client;
pub(crate) mod convert;
mod limits;
mod message;
mod request;
mod response;
mod stream;
mod tool;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_provider;
