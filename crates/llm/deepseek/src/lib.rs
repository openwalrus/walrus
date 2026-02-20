//! DeepSeek LLM provider

use llm::reqwest::{Client, header::HeaderMap};
pub use request::Request;

mod provider;
mod request;

/// The DeepSeek LLM provider
#[derive(Clone)]
pub struct DeepSeek {
    /// The HTTP client
    pub client: Client,

    /// The request headers
    headers: HeaderMap,
}
