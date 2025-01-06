//! Model loader

use crate::{Inference, TokenStream};
use anyhow::Result;
use candle_core::Device;
use ccore::{Manifest, TOKENIZER};
use hf_hub::api::sync::Api;
use std::fs::File;
use tokenizers::Tokenizer;

/// Huggingface model loader
///
/// TODO: embed the repo selection logic here
pub struct Loader {
    /// The HuggingFace API
    api: Api,

    /// The manifest of the model
    manifest: Manifest,
}

impl Loader {
    /// Load the model
    pub fn new(manifest: Manifest) -> Result<Self> {
        Ok(Self {
            manifest,
            api: Api::new()?,
        })
    }

    /// Load the tokenizer
    pub fn tokenizer(&self) -> Result<TokenStream> {
        let trepo = self.api.model(TOKENIZER.into());
        let tokenizer = Tokenizer::from_file(trepo.get(self.manifest.release.tokenizer())?)
            .map_err(|e| anyhow::anyhow!("failed to load tokenizer: {e}"))?;
        Ok(TokenStream::new(tokenizer))
    }

    /// Load the model
    pub fn model<M: Inference>(&self, device: &Device) -> Result<M> {
        let mrepo = self.api.model(self.manifest.release.repo()?.into());
        let model = mrepo.get(&self.manifest.release.model(self.manifest.quantization))?;
        let mut file = File::open(model)?;
        let model = M::gguf(device, &mut file)?;
        Ok(model)
    }
}
