//! Unified LLM Interface

pub mod cmd;

pub use deepseek::DeepSeek;
pub use ucore::{Chat, ChatMessage, Client, Config, LLM, Message, Response, StreamChunk};
