//! llama model interface

use std::fs;

use crate::{
    manifest::Manifest,
    util::{self, TokenOutputStream},
    Config, Message, Model,
};
use anyhow::Result;
use candle_core::{quantized::gguf_file, Device, Tensor};
use candle_transformers::{
    generation::{LogitsProcessor, Sampling},
    models::quantized_llama::{self, ModelWeights},
};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;

/// Llama model from by Meta
pub struct Llama {
    /// The config of the model
    config: Config,

    /// The tokenizer of the model
    tokenizer: Tokenizer,

    /// The device of the model
    device: Device,

    /// The model weights of the model
    model: ModelWeights,

    /// The logits processor of the model
    processor: LogitsProcessor,
}

impl Model for Llama {
    fn build(api: Api, config: Config, manifest: Manifest) -> Result<Self> {
        let trepo = api.model("clearloop/tokenizer".into());
        let tokenizer = Tokenizer::from_file(trepo.get(manifest.release.tokenizer())?)
            .map_err(|e| anyhow::anyhow!("failed to load tokenizer: {e}"))?;

        // load the model
        let mrepo = api.model(manifest.release.repo()?.into());
        let model = mrepo.get(&manifest.release.model(manifest.quantization))?;
        let mut file = fs::File::open(model)?;
        let model = gguf_file::Content::read(&mut file)?;
        let device = util::device(config.cpu)?;
        let model = ModelWeights::from_gguf(model, &mut file, &device)?;

        // load the logits processor
        let processor = {
            let temperature = config.temp.unwrap_or(0.6);
            let sampling = if temperature <= 0. {
                Sampling::ArgMax
            } else {
                match (config.top_k, config.top_p) {
                    (None, None) => Sampling::All { temperature },
                    (Some(k), None) => Sampling::TopK { k, temperature },
                    (None, Some(p)) => Sampling::TopP { p, temperature },
                    (Some(k), Some(p)) => Sampling::TopKThenTopP { k, p, temperature },
                }
            };
            LogitsProcessor::from_sampling(config.seed, sampling)
        };

        Ok(Self {
            config,
            tokenizer,
            device,
            model,
            processor,
        })
    }

    fn complete(&mut self, messages: &mut [Message]) -> Result<String> {
        let message = messages
            .first()
            .ok_or_else(|| anyhow::anyhow!("no messages"))?;
        let mut tos = TokenOutputStream::new(self.tokenizer.clone());
        let tokens = tos
            .tokenizer()
            .encode(message.content.clone(), true)
            .map_err(|e| anyhow::anyhow!("failed to encode message: {e}"))?;

        let to_sample = self.config.sample_len.saturating_sub(1);
        let mut prompt_tokens = tokens.get_ids().to_vec();
        if prompt_tokens.len() + to_sample > quantized_llama::MAX_SEQ_LEN - 10 {
            prompt_tokens = prompt_tokens[prompt_tokens.len().saturating_sub(to_sample)..].to_vec();
        }

        // process the prompt tokens
        let input = Tensor::new(prompt_tokens.as_slice(), &self.device)?.unsqueeze(0)?;
        let logits = self.model.forward(&input, 0)?;
        let mut next_token = self.processor.sample(&logits)?;

        let mut all_tokens = vec![next_token];
        let eos_token = *tos
            .tokenizer()
            .get_vocab(true)
            .get("</s>")
            .ok_or_else(|| anyhow::anyhow!("eos token not found"))?;

        let mut response = String::new();
        for index in 0..to_sample {
            let input = Tensor::new(&[next_token], &self.device)?;
            let logits = self
                .model
                .forward(&input, prompt_tokens.len() + index)?
                .squeeze(0)?;
            let logits = if self.config.repeat_penalty == 1. {
                logits
            } else {
                let start_at = all_tokens.len().saturating_sub(self.config.repeat_last_n);
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    self.config.repeat_penalty,
                    &all_tokens[start_at..],
                )?
            };
            next_token = self.processor.sample(&logits)?;
            all_tokens.push(next_token);

            if let Some(t) = tos.next_token(next_token)? {
                response += t.as_ref();
            }

            if next_token == eos_token {
                break;
            }
        }

        Ok(response)
    }

    fn tokenizer(&mut self) -> &mut Tokenizer {
        &mut self.tokenizer
    }

    fn embed(&mut self, _messages: Vec<String>) -> Result<Tensor> {
        todo!()
    }
}
