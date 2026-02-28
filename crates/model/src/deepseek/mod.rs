//! DeepSeek LLM provider

pub use request::Request;
use reqwest::{Client, header::HeaderMap};

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
