//! Token output stream

use crate::{Inference, Processor, Tokenizer};
use anyhow::Result;

/// Token output stream
pub struct TokenStream<'ts, I: Inference> {
    /// The current tokens
    cur_tokens: Vec<u32>,

    /// The end of stream token
    eos: u32,

    /// The initial response
    initial: Option<String>,

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
            eos: tokenizer
                .token(I::eos_token())
                .ok_or_else(|| anyhow::anyhow!("eos token not found"))?,
            initial: None,
            next: 0,
            pos: 0,
            processor,
            sampled: 0,
            tokenizer,
            weights,
        };

        this.sample_prompt(&prompt)?;
        Ok(this)
    }

    /// Sample the prompt
    fn sample_prompt(&mut self, prompt: &str) -> Result<()> {
        let tokens = self
            .tokenizer
            .prompt(&prompt)?
            .sample_len(self.processor.sample_len)
            .max_seq_len::<I>()
            .encode::<I>()?;

        self.pos = self.tokenizer.pos();
        for token in tokens.iter() {
            self.next = self
                .processor
                .sample_token(*token)
                .pos(self.pos)
                .sample(self.weights)?;

            self.cur_tokens.push(self.next);
            self.tokenizer.sampled(self.next);
            self.pos += 1;
        }

        if let Some(token) = self.tokenizer.next_token(self.next).ok().flatten() {
            self.initial = Some(token);
        }

        Ok(())
    }
}

impl<I: Inference> Iterator for TokenStream<'_, I> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.initial.take() {
            return Some(token);
        }

        if self.next == self.eos || self.sampled >= self.processor.sample_len {
            return None;
        }

        self.next = self
            .processor
            .sample_token(self.next)
            .cur_tokens(&self.cur_tokens)
            .pos(self.pos)
            .sample(self.weights)
            .ok()?;

        self.pos += 1;
        self.sampled += 1;
        self.cur_tokens.push(self.next);

        Some(
            self.tokenizer
                .next_token(self.next)
                .ok()
                .flatten()
                .unwrap_or_default(),
        )
    }
}
