//! LLM models

use crate::config::Config;
use anyhow::Result;
use ccore::{Manifest, Message};
use hf_hub::api::sync::Api;
pub use llama::Llama;

mod llama;

/// LLM interface
///
/// TODO: use trait or a single struct?
pub trait Model: Sized {
    /// Load the model from config.
    fn build(api: Api, config: Config, manifest: Manifest) -> Result<Self>;

    /// Complete the chat.
    ///
    /// TODO: use output stream
    fn complete(&mut self, _messages: &mut [Message]) -> Result<String> {
        anyhow::bail!("model does not support complete");
    }
}
