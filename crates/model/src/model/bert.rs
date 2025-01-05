//! bert model
#![allow(unused)]

use crate::{manifest::Manifest, util, Config, Message, Model};
use anyhow::Result;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert;
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::fs;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer};

const PYTORCH: &str = "pytorch_model.bin";
const SAFETENSORS: &str = "model.safetensors";

/// bert model interface for cydonia.
pub struct Bert {
    model: bert::BertModel,
    tokenizer: tokenizers::Tokenizer,
    device: Device,
}

impl Model for Bert {
    fn build(api: Api, config: Config, _manifest: Manifest) -> Result<Self> {
        let device = util::device(config.cpu)?;
        let repo = api.repo(Repo::with_revision(
            "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            RepoType::Model,
            config.revision.clone(),
        ));

        let builder = if config.pth {
            VarBuilder::from_pth(repo.get(PYTORCH)?, bert::DTYPE, &device)
        } else {
            unsafe {
                VarBuilder::from_mmaped_safetensors(&[repo.get(SAFETENSORS)?], bert::DTYPE, &device)
            }
        };

        Ok(Self {
            device,
            model: bert::BertModel::load(
                builder?,
                &serde_json::from_str(&fs::read_to_string(repo.get("config.json")?)?)?,
            )?,
            tokenizer: tokenizers::Tokenizer::from_file(repo.get("tokenizer.json")?)
                .map_err(|e| anyhow::anyhow!("failed to load tokenizer: {e}"))?,
        })
    }

    fn similar(
        &mut self,
        source: Message,
        mut messages: Vec<Message>,
        score: f32,
    ) -> Result<Vec<Message>> {
        let count = messages.len();
        messages.push(source);
        let sentences = messages.iter().map(|m| m.to_string()).collect::<Vec<_>>();
        self.ensure_padding_strategy(PaddingStrategy::BatchLongest);

        // embed the messages

        let mut embeddings = self.embed(sentences)?;
        tracing::trace!("generated embeddings: {:?}", embeddings.shape());

        let (_n_sentences, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
        tracing::trace!("pooled embeddings: {:?}", embeddings.shape());

        // detect the similar messages
        let mut similarities = vec![];
        let source_embedding = embeddings.get(count)?;
        for i in 0..count {
            let embedding = embeddings.get(i)?;
            let sum_ij = (&source_embedding * &embedding)?
                .sum_all()?
                .to_scalar::<f32>()?;
            let sum_i = (&source_embedding * &source_embedding)?
                .sum_all()?
                .to_scalar::<f32>()?;
            let sum_j = (&embedding * &embedding)?.sum_all()?.to_scalar::<f32>()?;
            let similarity = sum_ij / (sum_i * sum_j).sqrt();
            if similarity > score {
                similarities.push(messages.remove(i));
            }
        }

        Ok(similarities)
    }

    fn embed(&mut self, messages: Vec<String>) -> Result<Tensor> {
        let tokens = self
            .tokenizer
            .encode_batch(messages, true)
            .map_err(|e| anyhow::anyhow!("failed to encode tokens: {e}"))?;
        let token_ids = Tensor::stack(
            &tokens
                .iter()
                .map(|tokens| {
                    Ok(Tensor::new(
                        tokens.get_ids().to_vec().as_slice(),
                        &self.device,
                    )?)
                })
                .collect::<Result<Vec<_>>>()?,
            0,
        )?;

        let attention_mask = Tensor::stack(
            &tokens
                .iter()
                .map(|tokens| {
                    Ok(Tensor::new(
                        tokens.get_attention_mask().to_vec().as_slice(),
                        &self.device,
                    )?)
                })
                .collect::<Result<Vec<_>>>()?,
            0,
        )?;

        let token_type_ids = token_ids.zeros_like()?;
        tracing::trace!("running inference on batch {:?}", token_ids.shape());

        self.model
            .forward(&token_ids, &attention_mask, Some(&token_type_ids))
            .map_err(Into::into)
    }

    fn tokenizer(&mut self) -> &mut Tokenizer {
        &mut self.tokenizer
    }
}
