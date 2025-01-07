//! Model loader

use crate::{Inference, Tokenizer};
use anyhow::Result;
use candle_core::Device;
use ccore::{Release, TOKENIZER};
use hf_hub::api::sync::Api;
use std::fs::File;

/// Huggingface model loader
///
/// TODO: embed the repo selection logic here
pub struct Loader {
    /// The HuggingFace API
    api: Api,

    /// The manifest of the model
    release: Release,
}

impl Loader {
    /// Load the model
    pub fn new(release: Release) -> Result<Self> {
        Ok(Self {
            release,
            api: Api::new()?,
        })
    }

    /// Load the tokenizer
    pub fn tokenizer<I: Inference>(&self) -> Result<Tokenizer> {
        let trepo = self.api.model(TOKENIZER.into());
        let tokenizer = tokenizers::Tokenizer::from_file(trepo.get(self.release.tokenizer())?)
            .map_err(|e| anyhow::anyhow!("failed to load tokenizer: {e}"))?;
        Tokenizer::new::<I>(tokenizer)
    }

    /// Load the model
    pub fn model<M: Inference>(&self, device: &Device) -> Result<M> {
        let mrepo = self.api.model(self.release.repo().into());
        let model = mrepo.get(&self.release.model())?;
        let mut file = File::open(model)?;
        let model = M::gguf(device, &mut file)?;
        Ok(model)
    }
}
