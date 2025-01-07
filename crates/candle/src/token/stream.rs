//! Token output stream

use crate::{Inference, Processor, Tokenizer};
use anyhow::Result;

/// Token output stream
pub struct TokenStream<'ts, I: Inference> {
    /// The all tokens
    all: Vec<u32>,

    /// The end of stream token
    eos: u32,

    /// The index
    index: usize,

    /// The next token
    next: u32,

    /// The position
    pos: usize,

    /// The processor
    processor: &'ts mut Processor,

    /// The sample length
    sample: usize,

    /// The tokenizer
    tokenizer: &'ts mut Tokenizer,

    /// The model weights
    weights: &'ts mut I,
}

impl<'ts, I: Inference> TokenStream<'ts, I> {
    /// Create a new token stream
    pub fn new(
        weights: &'ts mut I,
        processor: &'ts mut Processor,
        tokenizer: &'ts mut Tokenizer,
        prompt: &'ts str,
    ) -> Result<Self> {
        let sample = processor.sample_len.saturating_sub(1);

        // TODO: adapt the eos token from weights spec
        let eos = tokenizer
            .token("</s>")
            .ok_or_else(|| anyhow::anyhow!("eos token not found"))?;

        // TODO: support split prompts
        let prompt_tokens = tokenizer
            .prompt(prompt)?
            .sample_len(sample)
            .max_seq_len::<I>()
            .encode()?;

        // process the prompt tokens
        let next = processor.sample_tokens(&prompt_tokens).sample(weights)?;
        Ok(Self {
            all: vec![next],
            eos,
            index: 0,
            next,
            pos: prompt_tokens.len(),
            processor,
            sample,
            tokenizer,
            weights,
        })
    }
}

impl<'ts, I: Inference> Iterator for TokenStream<'ts, I> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.sample || self.next == self.eos {
            return None;
        }

        self.next = self
            .processor
            .sample_tokens(&[self.next])
            .all_tokens(&self.all)
            .pos(self.pos + self.index)
            .sample(self.weights)
            .ok()?;

        self.all.push(self.next);
        self.index += 1;
        self.tokenizer.next_token(self.next).ok().flatten()
    }
}
