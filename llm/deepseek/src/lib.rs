//! The LLM provider

use ccore::{Client, reqwest::header::HeaderMap};
pub use request::Request;

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
