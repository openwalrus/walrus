//! Token stream handler

use crate::{Inference, Processor};
use anyhow::Result;
pub use {prompt::PromptBuilder, stream::TokenStream};

mod prompt;
mod stream;

/// A token stream handler
pub struct Tokenizer {
    /// The tokenizer
    tokenizer: tokenizers::Tokenizer,
    /// The full context including the tokens inferenced by the model
    /// and the users' input
    tokens: Vec<u32>,

    /// The end of stream token
    pub eos: u32,
}

impl Tokenizer {
    /// Create a new token stream
    pub fn new<I: Inference>(tokenizer: tokenizers::Tokenizer) -> Result<Self> {
        Ok(Self {
            tokens: Vec::new(),
            eos: tokenizer
                .get_vocab(true)
                .get(I::eos_token())
                .copied()
                .ok_or_else(|| anyhow::anyhow!("eos token not found"))?,
            tokenizer,
        })
    }

    /// Get the count of the tokens
    pub fn tokens(&self) -> usize {
        self.tokens.len()
    }

    /// Add a token to the context
    pub fn sampled(&mut self, tokens: &[u32]) {
        self.tokens.extend(tokens);
    }

    /// Embed a token to the context
    pub fn embed(&mut self, token: u32) -> Result<String> {
        match self.tokenizer.decode(&[token], true) {
            Ok(str) => {
                self.tokens.push(token);
                Ok(str)
            }
            Err(err) => anyhow::bail!("cannot decode: {err}"),
        }
    }

    /// Decode the tokens to string
    pub fn decode(&self, tokens: &[u32]) -> Result<String> {
        match self.tokenizer.decode(tokens, true) {
            Ok(str) => Ok(str),
            Err(err) => anyhow::bail!("cannot decode: {err}"),
        }
    }

    /// Encode the input text
    pub fn encode(&self, text: &str, special_tokens: bool) -> Result<Vec<u32>> {
        self.tokenizer
            .encode(text, special_tokens)
            .map(|e| e.get_ids().to_vec())
            .map_err(|e| anyhow::anyhow!("failed to encode: {e}"))
    }

    /// Encode the prompt string
    pub fn prompt<'p>(&'p mut self, text: &'p str) -> Result<PromptBuilder<'p>> {
        Ok(PromptBuilder::new(self, text))
    }

    /// Get token from the input string
    pub fn token(&self, token_s: &str) -> Option<u32> {
        self.tokenizer.get_vocab(true).get(token_s).copied()
    }

    /// Get the token stream
    pub fn stream<'ts, I: Inference>(
        &'ts mut self,
        weights: &'ts mut I,
        processor: &'ts mut Processor,
        prompt: String,
    ) -> Result<TokenStream<'ts, I>> {
        TokenStream::new(weights, processor, self, prompt)
    }
}
