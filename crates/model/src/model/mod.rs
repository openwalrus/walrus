//! LLM models

use crate::{chat::Message, config::Config};
use anyhow::Result;
pub use bert::Bert;
use candle_core::{DType, Tensor};
use hf_hub::api::sync::Api;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer};

pub mod bert;

/// LLM interface
///
/// TODO: use trait or a single struct?
pub trait Model: Sized {
    /// The hugging face repo of the model.
    const REPO: &str;

    /// The type of the model.
    const DTYPE: DType;

    /// Load the model from config.
    fn build(api: Api, config: Config) -> Result<Self>;

    /// Complete the chat.
    fn complete(&mut self, _messages: &mut [Message]) -> Result<()> {
        anyhow::bail!("model {} does not support complete", Self::REPO);
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
        anyhow::bail!("model {} does not support similar messages", Self::REPO);
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
