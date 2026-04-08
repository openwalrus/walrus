//! Free-function accessors over `crabllm_core` wire types.
//!
//! These bridge the ergonomic gap left by the old `wcore::StreamChunk` and
//! `wcore::Response` convenience methods. Everything here is a one-line
//! navigation into the crabllm type; the functions exist to keep call sites
//! readable and to centralize the "first choice" convention.
//!
//! Upstream convenience helpers tracked in crabtalk/crabllm#46 — if they
//! land, these can be replaced with upstream method calls.

use crabllm_core::{
    ChatCompletionChunk, ChatCompletionResponse, FinishReason, Message, ToolCall, ToolCallDelta,
};

/// Text content of the first choice's delta, or `None` if absent/empty.
pub fn chunk_content(chunk: &ChatCompletionChunk) -> Option<&str> {
    chunk
        .choices
        .first()
        .and_then(|c| c.delta.content.as_deref())
        .filter(|s| !s.is_empty())
}

/// Reasoning content of the first choice's delta, or `None` if absent/empty.
pub fn chunk_reasoning(chunk: &ChatCompletionChunk) -> Option<&str> {
    chunk
        .choices
        .first()
        .and_then(|c| c.delta.reasoning_content.as_deref())
        .filter(|s| !s.is_empty())
}

/// Tool-call deltas of the first choice's delta, or an empty slice if absent.
pub fn chunk_tool_calls(chunk: &ChatCompletionChunk) -> &[ToolCallDelta] {
    chunk
        .choices
        .first()
        .and_then(|c| c.delta.tool_calls.as_deref())
        .unwrap_or(&[])
}

/// Finish reason of the first choice, or `None` if not yet set.
pub fn chunk_finish_reason(chunk: &ChatCompletionChunk) -> Option<&FinishReason> {
    chunk.choices.first().and_then(|c| c.finish_reason.as_ref())
}

/// The message of the first choice in a non-streaming response.
pub fn response_message(resp: &ChatCompletionResponse) -> Option<&Message> {
    resp.choices.first().map(|c| &c.message)
}

/// Text content of the first choice's message, or `None` if absent/empty/non-string.
pub fn response_content(resp: &ChatCompletionResponse) -> Option<&str> {
    response_message(resp)
        .and_then(|m| m.content.as_ref())
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
}

/// Tool calls on the first choice's message, or an empty slice if absent.
pub fn response_tool_calls(resp: &ChatCompletionResponse) -> &[ToolCall] {
    resp.choices
        .first()
        .and_then(|c| c.message.tool_calls.as_deref())
        .unwrap_or(&[])
}
