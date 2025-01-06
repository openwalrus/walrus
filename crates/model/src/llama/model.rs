//! llama model interface

use crate::util::TokenOutputStream;
use anyhow::Result;
use candle::{Loader, Processor, ProcessorConfig};
use candle_transformers::models::quantized_llama::{self, ModelWeights};
use ccore::{Manifest, Message};
use std::io::Write;
use tokenizers::Tokenizer;

/// Llama model from by Meta
pub struct Llama {
    /// The tokenizer of the model
    tokenizer: Tokenizer,

    /// The model weights of the model
    model: ModelWeights,

    /// The logits processor of the model
    processor: Processor,
}

impl Llama {
    /// Build the llama model
    pub fn build(config: ProcessorConfig, manifest: Manifest) -> Result<Self> {
        let loader = Loader::new(manifest)?;
        let tokenizer = loader.tokenizer()?;
        let processor = config.build();
        let model = loader.model::<ModelWeights>(&processor.device)?;

        Ok(Self {
            tokenizer,
            model,
            processor,
        })
    }

    /// Complete the chat
    pub fn complete(&mut self, messages: &mut [Message]) -> Result<String> {
        let message = messages
            .first()
            .ok_or_else(|| anyhow::anyhow!("no messages"))?;
        let mut tos = TokenOutputStream::new(self.tokenizer.clone());
        let tokens = tos
            .tokenizer()
            .encode(message.content.clone(), true)
            .map_err(|e| anyhow::anyhow!("failed to encode message: {e}"))?;

        let to_sample = self.processor.sample_len.saturating_sub(1);
        let mut prompt_tokens = tokens.get_ids().to_vec();
        if prompt_tokens.len() + to_sample > quantized_llama::MAX_SEQ_LEN - 10 {
            prompt_tokens = prompt_tokens[prompt_tokens.len().saturating_sub(to_sample)..].to_vec();
        }

        // process the prompt tokens
        let mut next_token = self
            .processor
            .sample_tokens(&prompt_tokens)
            .sample(&mut self.model)?;

        // process the tokens
        let mut all_tokens = vec![next_token];
        let eos_token = *tos
            .tokenizer()
            .get_vocab(true)
            .get("</s>")
            .ok_or_else(|| anyhow::anyhow!("eos token not found"))?;

        let response = String::new();
        let pos = prompt_tokens.len();
        for index in 0..to_sample {
            next_token = self
                .processor
                .sample_tokens(&[next_token])
                .all_tokens(&all_tokens)
                .pos(pos + index)
                .sample(&mut self.model)?;

            all_tokens.push(next_token);
            if let Some(t) = tos.next_token(next_token)? {
                print!("{t}");
                std::io::stdout().flush()?;
            }

            if next_token == eos_token {
                break;
            }
        }

        println!();
        Ok(response)
    }
}
