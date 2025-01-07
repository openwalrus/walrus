//! Token output stream

use crate::{Inference, Processor, Tokenizer};
use anyhow::Result;

/// Token output stream
pub struct TokenStream<'ts, I: Inference> {
    /// The all tokens
    all: Vec<u32>,

    /// The end of stream token
    eos: u32,

    /// The next token
    next: u32,

    /// The position
    pos: usize,

    /// The processor
    processor: &'ts mut Processor,

    /// The sample
    sample: Vec<u32>,

    /// The target sample length limit
    to_sample: usize,

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
        let to_sample = processor.sample_len.saturating_sub(1);
        let eos = tokenizer
            // This is specified by llama3 spec
            .token("<|end_of_text|>")
            .ok_or_else(|| anyhow::anyhow!("eos token not found"))?;

        // TODO: support split prompts
        let prompt_tokens = tokenizer
            .prompt(prompt)?
            .sample_len(to_sample)
            .max_seq_len::<I>()
            .encode()?;

        Ok(Self {
            all: vec![],
            eos,
            next: 0,
            pos: 0,
            processor,
            sample: prompt_tokens,
            to_sample,
            tokenizer,
            weights,
        })
    }
}

impl<I: Inference> Iterator for TokenStream<'_, I> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.to_sample || self.next == self.eos {
            return None;
        }

        self.next = self
            .processor
            .sample_tokens(&self.sample)
            .all_tokens(&self.all)
            .pos(self.pos)
            .sample(self.weights)
            .ok()?;

        self.all.push(self.next);
        self.pos += self.sample.len();
        self.sample = vec![self.next];
        self.tokenizer.next_token(self.next).ok().flatten()
    }
}
