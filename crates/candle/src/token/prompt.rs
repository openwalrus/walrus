//! Prompt builder

use crate::{Inference, Tokenizer};
use anyhow::Result;

/// Prompt builder
pub struct PromptBuilder<'t> {
    /// The token stream
    tos: &'t mut Tokenizer,

    /// The text
    text: &'t str,

    /// The special tokens
    special_tokens: bool,

    /// The sample length
    sample_len: Option<usize>,

    /// The max sequence length
    max_seq_len: Option<usize>,
}

impl<'t> PromptBuilder<'t> {
    /// Create a new prompt builder
    pub fn new(tos: &'t mut Tokenizer, text: &'t str) -> Self {
        Self {
            tos,
            text,
            special_tokens: false,
            sample_len: None,
            max_seq_len: None,
        }
    }

    /// Set the special tokens
    pub fn special_tokens(mut self, special_tokens: bool) -> Self {
        self.special_tokens = special_tokens;
        self
    }

    /// Set the sample length
    pub fn sample_len(mut self, sample_len: usize) -> Self {
        self.sample_len = Some(sample_len);
        self
    }

    /// Set the max sequence length
    pub fn max_seq_len<M: Inference>(mut self) -> Self {
        self.max_seq_len = Some(M::MAX_SEQ_LEN);
        self
    }

    /// Encode the text to tokens
    pub fn encode<I: Inference>(self) -> Result<Vec<u32>> {
        let mut tokens = self.tos.encode(self.text, self.special_tokens)?;
        if let (Some(max_seq_len), Some(sample_len)) = (self.max_seq_len, self.sample_len) {
            let eos_token_len = I::eos_token().len();
            if tokens.len() + sample_len > max_seq_len.saturating_sub(eos_token_len) {
                let to_remove = tokens.len() + sample_len + eos_token_len - max_seq_len;
                tokens = tokens[tokens.len().saturating_sub(to_remove)..].to_vec();
            }
        }

        self.tos.sampled(&tokens);
        Ok(tokens)
    }
}
