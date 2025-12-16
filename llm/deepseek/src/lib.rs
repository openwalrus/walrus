//! The LLM provider

pub use request::Request;
use ucore::{Client, reqwest::header::HeaderMap};

mod llm;
mod request;

/// The DeepSeek LLM provider

#[derive(Clone)]
pub struct DeepSeek {
    /// The HTTP client
    pub client: Client,

    /// The request headers
    headers: HeaderMap,
}
