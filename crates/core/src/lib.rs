//! Core abstractions for Unified LLM Interface

pub use {
    agent::Agent,
    chat::{Chat, ChatMessage},
    config::{Config, General},
    message::{Message, Role},
    provider::LLM,
    reqwest::{self, Client},
    response::{
        Choice, CompletionTokensDetails, FinishReason, LogProb, LogProbs, Response,
        ResponseMessage, TopLogProb, Usage,
    },
    stream::{Delta, StreamChoice, StreamChunk},
    tool::{FunctionCall, Tool, ToolCall, ToolChoice},
};

mod agent;
mod chat;
mod config;
mod message;
mod provider;
mod response;
mod stream;
mod tool;
