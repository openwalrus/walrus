//! Logit processor

use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use std::ops::{Deref, DerefMut};
pub use {config::ProcessorConfig, sample::SampleBuilder};

mod config;
mod sample;

/// Candle logit processor builder
pub struct Processor {
    /// The logit processor
    processor: LogitsProcessor,

    /// The device
    pub device: Device,

    /// The repeat penalty
    repeat_penalty: f32,

    /// The repeat last n
    repeat_last_n: usize,

    /// The sample length
    pub sample_len: usize,
}

impl Processor {
    /// Create a new logit processor builder
    pub fn builder() -> ProcessorConfig {
        ProcessorConfig::default()
    }

    /// Create a new tensor
    pub fn tensor(&self, tokens: &[u32], unsqueeze: usize) -> anyhow::Result<Tensor> {
        Tensor::new(tokens, &self.device)?
            .unsqueeze(unsqueeze)
            .map_err(|e| anyhow::anyhow!("failed to unsqueeze: {e}"))
    }

    /// Sample tokens
    pub fn sample_token(&mut self, token: u32) -> SampleBuilder<'_> {
        SampleBuilder::new(self, token)
    }

    /// Apply repeat penalty
    fn repeat_penalty(&self, logits: Tensor, tokens: &[u32]) -> anyhow::Result<Tensor> {
        if self.repeat_penalty == 1. {
            Ok(logits)
        } else {
            let start_at = tokens.len().saturating_sub(self.repeat_last_n);
            candle_transformers::utils::apply_repeat_penalty(
                &logits,
                self.repeat_penalty,
                &tokens[start_at..],
            )
            .map_err(|e| anyhow::anyhow!("failed to apply repeat penalty: {e}"))
        }
    }
}

impl Deref for Processor {
    type Target = LogitsProcessor;

    fn deref(&self) -> &Self::Target {
        &self.processor
    }
}

impl DerefMut for Processor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.processor
    }
}
