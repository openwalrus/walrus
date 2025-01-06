//! Prompt builder

use crate::{Inference, TokenStream};
use anyhow::Result;

/// Prompt builder
pub struct PromptBuilder<'t> {
    /// The token stream
    tos: &'t TokenStream,

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
    pub fn new(tos: &'t TokenStream, text: &'t str) -> Self {
        Self {
            tos,
            text,
            special_tokens: true,
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
    pub fn encode(self) -> Result<Vec<u32>> {
        let mut tokens = self.tos.encode(self.text, self.special_tokens)?;
        if let (Some(max_seq_len), Some(sample_len)) = (self.max_seq_len, self.sample_len) {
            // NOTE: we need to subtract 10 to account for the eos token
            if tokens.len() + sample_len > max_seq_len.saturating_sub(10) {
                // TODO: handle the case where the tokens are too long
                tokens = tokens[tokens.len().saturating_sub(sample_len)..].to_vec();
            }
        }

        Ok(tokens)
    }
}
