//! LLM models

use crate::{chat::Message, config::Config, manifest::Manifest};
use anyhow::Result;
use candle_core::Tensor;
use hf_hub::api::sync::Api;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer};
pub use {bert::Bert, llama::Llama};

pub mod bert;
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

    /// Embed the messages.
    fn embed(&mut self, messages: Vec<String>) -> Result<Tensor>;

    /// Get the similar messages
    fn similar(
        &mut self,
        _source: Message,
        _messages: Vec<Message>,
        _score: f32,
    ) -> Result<Vec<Message>> {
        anyhow::bail!("model does not support similar messages");
    }

    /// Get the tokenizer
    fn tokenizer(&mut self) -> &mut Tokenizer;

    /// Ensure padding strategy
    fn ensure_padding_strategy(&mut self, strategy: PaddingStrategy) {
        let tokenizer = self.tokenizer();
        if let Some(pp) = tokenizer.get_padding_mut() {
            pp.strategy = strategy;
        } else {
            let pp = PaddingParams {
                strategy,
                ..Default::default()
            };
            tokenizer.with_padding(Some(pp));
        }
    }
}
