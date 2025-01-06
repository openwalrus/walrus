//! Cydonia inference interface
use anyhow::Result;
use candle_core::Tensor;
use candle_transformers::models::quantized_llama;

/// The inference interface for language models
pub trait Inference {
    /// The forward pass of the model
    fn forward(&mut self, input: &Tensor, squeeze: usize) -> Result<Tensor>;
}

impl Inference for quantized_llama::ModelWeights {
    fn forward(&mut self, input: &Tensor, squeeze: usize) -> Result<Tensor> {
        quantized_llama::ModelWeights::forward(self, input, squeeze)
            .map_err(|e| anyhow::anyhow!("failed to forward: {e}"))
    }
}
