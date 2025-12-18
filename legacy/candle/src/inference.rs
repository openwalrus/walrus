//! Cydonia inference interface
use anyhow::Result;
use candle_core::{Device, Tensor, quantized::gguf_file::Content};
use candle_transformers::models::quantized_llama;
use model::{Message, chat};
use std::fs::File;

/// The inference interface for language models
pub trait Inference: Sized {
    /// The max sequence length
    const MAX_SEQ_LEN: usize;

    /// The formatter for the model
    type Formatter: chat::Formatter;

    /// The end of stream token
    fn eos_token() -> &'static str {
        <Self::Formatter as chat::Formatter>::EOS_TOKEN
    }

    /// Format the messages into a prompt
    fn prompt(messages: &[Message]) -> Result<String> {
        <Self::Formatter as chat::Formatter>::format(messages)
    }

    /// Complete the messages
    fn complete(messages: &[Message]) -> Result<String> {
        <Self::Formatter as chat::Formatter>::complete(messages)
    }

    /// Load model from gguf file
    fn gguf(device: &Device, file: &mut File) -> Result<Self>;

    /// The forward pass of the model
    fn forward(&mut self, input: &Tensor, squeeze: usize) -> Result<Tensor>;
}

impl Inference for quantized_llama::ModelWeights {
    const MAX_SEQ_LEN: usize = quantized_llama::MAX_SEQ_LEN;

    type Formatter = chat::Llama3;

    fn gguf(device: &Device, file: &mut File) -> Result<Self> {
        let content = Content::read(file)?;
        let model = Self::from_gguf(content, file, device)?;
        Ok(model)
    }

    fn forward(&mut self, input: &Tensor, pos: usize) -> Result<Tensor> {
        quantized_llama::ModelWeights::forward(self, input, pos)
            .map_err(|e| anyhow::anyhow!("failed to forward: {e}"))
    }
}
