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

    /// The tokens
    tokens: Vec<u32>,
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
            tokens: vec![],
        })
    }

    /// Complete the chat
    pub fn complete<'ts>(
        &'ts mut self,
        messages: &[Message],
        complete: bool,
    ) -> Result<TokenStream<'ts, I>> {
        let formatted = if complete {
            I::complete_format(messages)?
        } else {
            I::format(messages)?
        };

        self.tokenizer.stream(
            &mut self.weights,
            &mut self.processor,
            &mut self.tokens,
            formatted,
        )
    }
}
