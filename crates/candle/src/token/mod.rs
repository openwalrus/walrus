//! Token stream handler

use crate::{Inference, Processor};
use anyhow::Result;
pub use {prompt::PromptBuilder, stream::TokenStream};

mod prompt;
mod stream;

/// A token stream handler
pub struct Tokenizer {
    tokenizer: tokenizers::Tokenizer,
    tokens: Vec<u32>,
    prev_index: usize,
    current_index: usize,
}

impl Tokenizer {
    /// Create a new token stream
    pub fn new(tokenizer: tokenizers::Tokenizer) -> Self {
        Self {
            tokenizer,
            tokens: Vec::new(),
            prev_index: 0,
            current_index: 0,
        }
    }

    /// Clear the token stream
    pub fn clear(&mut self) {
        self.tokens.clear();
        self.prev_index = 0;
        self.current_index = 0;
    }

    fn decode(&self, tokens: &[u32]) -> Result<String> {
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

    /// Get the next token
    ///
    /// <https://github.com/huggingface/text-generation-inference/blob/5ba53d44a18983a4de32d122f4cb46f4a17d9ef6/server/text_generation_server/models/model.py#L68>
    pub fn next_token(&mut self, token: u32) -> Result<Option<String>> {
        let prev_text = if self.tokens.is_empty() {
            String::new()
        } else {
            let tokens = &self.tokens[self.prev_index..self.current_index];
            self.decode(tokens)?
        };

        self.tokens.push(token);
        let text = self.decode(&self.tokens[self.prev_index..])?;
        if text.len() > prev_text.len() && text.chars().last().unwrap().is_alphanumeric() {
            let text = text.split_at(prev_text.len());
            self.prev_index = self.current_index;
            self.current_index = self.tokens.len();
            Ok(Some(text.1.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Encode the prompt string
    pub fn prompt<'p>(&'p self, text: &'p str) -> Result<PromptBuilder<'p>> {
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
        tokens: &'ts mut Vec<u32>,
        prompt: String,
    ) -> Result<TokenStream<'ts, I>> {
        TokenStream::new(weights, processor, self, prompt, tokens)
    }
}

impl From<tokenizers::Tokenizer> for Tokenizer {
    fn from(tokenizer: tokenizers::Tokenizer) -> Self {
        Self::new(tokenizer)
    }
}
