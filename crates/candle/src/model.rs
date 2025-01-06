//! Model interface

use crate::{Inference, Loader, Processor, ProcessorConfig, TokenStream};
use anyhow::Result;
use ccore::{Manifest, Message};
use std::io::Write;

/// Language Model interface
pub struct Model<I: Inference> {
    /// The tokenizer of the model
    tokenizer: TokenStream,

    /// The weights of the model
    weights: I,

    /// The logits processor of the model
    processor: Processor,
}

impl<I: Inference> Model<I> {
    /// Create a new model
    pub fn new(config: ProcessorConfig, manifest: Manifest) -> Result<Self> {
        let loader = Loader::new(manifest)?;
        let tokenizer = loader.tokenizer()?;
        let processor = config.build();
        let weights = loader.model::<I>(&processor.device)?;

        Ok(Self {
            tokenizer,
            weights,
            processor,
        })
    }

    /// Complete the chat
    pub fn complete(&mut self, messages: &mut [Message]) -> Result<String> {
        let message = messages
            .first()
            .ok_or_else(|| anyhow::anyhow!("no messages"))?;

        let to_sample = self.processor.sample_len.saturating_sub(1);
        let prompt_tokens = self
            .tokenizer
            .prompt(&message.content)?
            .sample_len(to_sample)
            .max_seq_len::<I>()
            .encode()?;

        // process the prompt tokens
        let mut next_token = self
            .processor
            .sample_tokens(&prompt_tokens)
            .sample(&mut self.weights)?;

        // process the tokens
        let mut all_tokens = vec![next_token];
        let eos_token = self
            .tokenizer
            .token("</s>")
            .ok_or_else(|| anyhow::anyhow!("eos token not found"))?;

        let response = String::new();
        let pos = prompt_tokens.len();
        for index in 0..to_sample {
            next_token = self
                .processor
                .sample_tokens(&[next_token])
                .all_tokens(&all_tokens)
                .pos(pos + index)
                .sample(&mut self.weights)?;

            all_tokens.push(next_token);
            if let Some(t) = self.tokenizer.next_token(next_token)? {
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
