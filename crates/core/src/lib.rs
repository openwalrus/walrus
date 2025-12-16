//! Core abstractions for Unified LLM Interface

pub use {
    chat::{Chat, ChatMessage},
    config::Config,
    message::{Message, Role},
    provider::LLM,
    reqwest::{self, Client},
    response::{
        Choice, CompletionTokensDetails, FinishReason, LogProb, LogProbs, Response,
        ResponseMessage, TopLogProb, Usage,
    },
    stream::{Delta, StreamChoice, StreamChunk},
    template::Template,
    tool::{FunctionCall, Tool, ToolCall, ToolChoice},
};

mod chat;
mod config;
mod message;
mod provider;
mod response;
mod stream;
mod template;
mod tool;
