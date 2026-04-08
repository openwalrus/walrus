//! Unified LLM interface types and the `Model<P>` wrapper.
//!
//! Provides the wcore-typed types used by `Agent` and `Runtime`:
//! `Message`, `Request`, `Response`, `StreamChunk`, `Tool`, plus the
//! `Model<P>` newtype that wraps any `crabllm_core::Provider` and exposes
//! `send`/`stream` over wcore types.
//!
//! During the #144 migration this module also hosts the new crabllm-typed
//! surface (`HistoryEntry`, the new `MessageBuilder` in `builder`, and the
//! free-function `accessors` helpers) that will replace the wcore types once
//! all call sites are flipped.

pub use accessors::{
    chunk_content, chunk_finish_reason, chunk_reasoning, chunk_tool_calls, response_content,
    response_message, response_tool_calls,
};
pub use client::Model;
pub use history::{HistoryEntry, estimate_tokens as estimate_history_tokens};
pub use limits::default_context_limit;
pub use message::{Message, MessageBuilder, Role, estimate_tokens};
pub use request::Request;
pub use response::{
    Choice, CompletionMeta, CompletionTokensDetails, Delta, FinishReason, Response, Usage,
};
pub use stream::StreamChunk;
pub use tool::{FunctionCall, Tool, ToolCall, ToolChoice};

mod accessors;
pub mod builder;
mod client;
pub(crate) mod convert;
mod history;
mod limits;
mod message;
mod request;
mod response;
mod stream;
mod tool;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_provider;
