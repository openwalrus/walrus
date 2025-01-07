//! Token output stream

use crate::{Inference, Processor, Tokenizer};
use anyhow::Result;

/// Token output stream
pub struct TokenStream<'ts, I: Inference> {
    /// The current tokens
    cur_tokens: Vec<u32>,

    /// The next token
    next: u32,

    /// The position
    pos: usize,

    /// The processor
    processor: &'ts mut Processor,

    /// The sampled tokens
    sampled: usize,

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
        prompt: String,
    ) -> Result<Self> {
        let mut this = Self {
            cur_tokens: vec![],
            pos: tokenizer.len(),
            next: 0,
            sampled: 0,
            processor,
            tokenizer,
            weights,
        };

        this.sample_prompt(&prompt)?;
        Ok(this)
    }

    /// Sample the prompt
    ///
    /// This function should only be called on the start of the stream.
    fn sample_prompt(&mut self, prompt: &str) -> Result<()> {
        let tokens = self
            .tokenizer
            .prompt(&prompt)?
            .sample_len(self.processor.sample_len)
            .max_seq_len::<I>()
            .encode::<I>()?;

        for token in tokens.iter() {
            self.sample_token(*token)?;
        }

        Ok(())
    }

    /// Sample a token
    fn sample_token(&mut self, token: u32) -> Result<()> {
        self.next = self
            .processor
            .sample_token(token)
            .cur_tokens(&self.cur_tokens)
            .pos(self.pos)
            .sample(self.weights)?;

        self.cur_tokens.push(self.next);
        self.pos += 1;
        Ok(())
    }
}

impl<I: Inference> Iterator for TokenStream<'_, I> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.tokenizer.len() {
            return self.tokenizer.embed(self.next).ok();
        }

        if self.next == self.tokenizer.eos || self.sampled >= self.processor.sample_len {
            return None;
        }

        self.sample_token(self.next).ok()?;
        self.sampled += 1;
        self.tokenizer.embed(self.next).ok()
    }
}
