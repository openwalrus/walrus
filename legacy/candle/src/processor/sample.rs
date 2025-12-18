//! Cydonia sample builder
use crate::{Inference, Processor};

/// Sample builder
pub struct SampleBuilder<'s> {
    token: u32,
    unsqueeze: usize,
    pos: usize,
    squeeze: usize,
    cur_tokens: &'s [u32],
    processor: &'s mut Processor,
}

impl<'s> SampleBuilder<'s> {
    /// Create a new sample builder
    pub fn new(processor: &'s mut Processor, token: u32) -> Self {
        Self {
            token,
            unsqueeze: 0,
            pos: 0,
            squeeze: 0,
            cur_tokens: &[],
            processor,
        }
    }

    /// Set the all tokens
    pub fn cur_tokens(mut self, cur_tokens: &'s [u32]) -> Self {
        self.cur_tokens = cur_tokens;
        self
    }

    /// Set the unsqueeze
    pub fn unsqueeze(mut self, unsqueeze: usize) -> Self {
        self.unsqueeze = unsqueeze;
        self
    }

    /// Set the pos
    pub fn pos(mut self, pos: usize) -> Self {
        self.pos = pos;
        self
    }

    /// Set the squeeze
    pub fn squeeze(mut self, squeeze: usize) -> Self {
        self.squeeze = squeeze;
        self
    }

    /// Build the sample
    pub fn sample(self, model: &mut impl Inference) -> anyhow::Result<u32> {
        let input = self.processor.tensor(&[self.token], self.unsqueeze)?;
        let mut logits = model.forward(&input, self.pos)?.squeeze(self.squeeze)?;
        if !self.cur_tokens.is_empty() {
            logits = self.processor.repeat_penalty(logits, self.cur_tokens)?;
        }

        self.processor
            .sample(&logits)
            .map_err(|e| anyhow::anyhow!("failed to sample: {e}"))
    }
}
