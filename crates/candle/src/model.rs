//! Model interface

use crate::{Inference, Loader, Processor, ProcessorConfig, TokenStream, Tokenizer};
use anyhow::Result;
use ccore::{Message, Release};

/// Language Model interface
pub struct Model<I: Inference> {
    /// The tokenizer of the model
    tokenizer: Tokenizer,

    /// The weights of the model
    weights: I,

    /// The logits processor of the model
    processor: Processor,
}

impl<I: Inference> Model<I> {
    /// Create a new model
    pub fn new(config: ProcessorConfig, release: Release) -> Result<Self> {
        let loader = Loader::new(release)?;
        let tokenizer = loader.tokenizer()?;
        let processor = config.build();
        let weights = loader.model::<I>(&processor.device)?;

        Ok(Self {
            tokenizer,
            weights,
            processor,
        })
    }

    /// Complete the chat
    pub fn complete<'ts>(
        &'ts mut self,
        messages: &'ts mut [Message],
    ) -> Result<TokenStream<'ts, I>> {
        let message = messages
            .first()
            .ok_or_else(|| anyhow::anyhow!("no messages"))?;

        self.tokenizer
            .stream(&mut self.weights, &mut self.processor, &message.content)
    }
}
