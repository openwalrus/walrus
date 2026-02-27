//! SSE event parsing for the Anthropic streaming Messages API.
//!
//! Anthropic streaming events differ from OpenAI's format:
//! - `message_start` — initial message metadata
//! - `content_block_start` — begin a content block (text or tool_use)
//! - `content_block_delta` — incremental content (text_delta or input_json_delta)
//! - `content_block_stop` — end of a content block
//! - `message_delta` — final stop_reason and usage
//! - `message_stop` — end of message

use compact_str::CompactString;
use llm::{
    Choice, CompletionMeta, CompletionTokensDetails, Delta, FinishReason, FunctionCall,
    StreamChunk, ToolCall, Usage,
};
use serde::Deserialize;

/// A raw SSE event from the Anthropic streaming API.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    /// Initial message metadata.
    #[serde(rename = "message_start")]
    MessageStart { message: MessageMeta },
    /// Begin a content block.
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    /// Incremental content within a block.
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: BlockDelta },
    /// End of a content block.
    #[serde(rename = "content_block_stop")]
    ContentBlockStop {},
    /// Final message delta (stop reason + usage).
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaBody, usage: MessageDeltaUsage },
    /// End of message.
    #[serde(rename = "message_stop")]
    MessageStop,
    /// Ping (keep-alive).
    #[serde(rename = "ping")]
    Ping,
    /// Catch-all for unknown event types.
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct MessageMeta {
    pub id: CompactString,
    pub model: CompactString,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: CompactString, name: CompactString },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum BlockDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
pub struct MessageDeltaBody {
    pub stop_reason: Option<CompactString>,
}

#[derive(Debug, Deserialize)]
pub struct MessageDeltaUsage {
    pub output_tokens: u32,
}

impl Event {
    /// Convert this Anthropic event to a walrus `StreamChunk`.
    /// Returns `None` for events that don't produce output (ping, stop, unknown).
    pub fn into_chunk(self) -> Option<StreamChunk> {
        match self {
            Self::MessageStart { message } => Some(StreamChunk {
                meta: CompletionMeta {
                    id: message.id,
                    object: "chat.completion.chunk".into(),
                    model: message.model,
                    ..Default::default()
                },
                ..Default::default()
            }),
            Self::ContentBlockStart {
                content_block: ContentBlock::Text { text },
                ..
            } => {
                if text.is_empty() {
                    None
                } else {
                    Some(StreamChunk {
                        choices: vec![Choice {
                            delta: Delta {
                                content: Some(text),
                                ..Default::default()
                            },
                            ..Default::default()
                        }],
                        ..Default::default()
                    })
                }
            }
            Self::ContentBlockStart {
                index,
                content_block: ContentBlock::ToolUse { id, name },
            } => Some(StreamChunk {
                choices: vec![Choice {
                    delta: Delta {
                        tool_calls: Some(vec![ToolCall {
                            id,
                            index,
                            call_type: "function".into(),
                            function: FunctionCall {
                                name,
                                arguments: String::new(),
                            },
                        }]),
                        ..Default::default()
                    },
                    ..Default::default()
                }],
                ..Default::default()
            }),
            Self::ContentBlockDelta {
                delta: BlockDelta::TextDelta { text },
                ..
            } => Some(StreamChunk {
                choices: vec![Choice {
                    delta: Delta {
                        content: Some(text),
                        ..Default::default()
                    },
                    ..Default::default()
                }],
                ..Default::default()
            }),
            Self::ContentBlockDelta {
                index,
                delta: BlockDelta::InputJsonDelta { partial_json },
            } => Some(StreamChunk {
                choices: vec![Choice {
                    delta: Delta {
                        tool_calls: Some(vec![ToolCall {
                            index,
                            function: FunctionCall {
                                arguments: partial_json,
                                ..Default::default()
                            },
                            ..Default::default()
                        }]),
                        ..Default::default()
                    },
                    ..Default::default()
                }],
                ..Default::default()
            }),
            Self::MessageDelta { delta, usage } => {
                let reason = delta.stop_reason.as_deref().map(|r| match r {
                    "end_turn" | "stop" => FinishReason::Stop,
                    "max_tokens" => FinishReason::Length,
                    "tool_use" => FinishReason::ToolCalls,
                    _ => FinishReason::Stop,
                });
                Some(StreamChunk {
                    choices: vec![Choice {
                        finish_reason: reason,
                        ..Default::default()
                    }],
                    usage: Some(Usage {
                        prompt_tokens: 0,
                        completion_tokens: usage.output_tokens,
                        total_tokens: usage.output_tokens,
                        prompt_cache_hit_tokens: None,
                        prompt_cache_miss_tokens: None,
                        completion_tokens_details: Some(CompletionTokensDetails {
                            reasoning_tokens: None,
                        }),
                    }),
                    ..Default::default()
                })
            }
            Self::ContentBlockStop {}
            | Self::MessageStop
            | Self::Ping
            | Self::Unknown => None,
        }
    }
}
