//! Local LLM provider via mistralrs (DD#67).
//!
//! Wraps `mistralrs::Model` for native on-device inference.
//! No HTTP transport — inference runs in-process.
//! Provides per-builder constructors: `from_text()`, `from_gguf()`,
//! `from_vision()`. All use the walrus model cache directory.

use std::path::PathBuf;
use std::sync::Arc;

mod provider;

/// Local LLM provider wrapping a mistralrs `Model`.
#[derive(Clone)]
pub struct Local {
    model: Arc<mistralrs::Model>,
}

impl Local {
    /// Construct from a pre-built mistralrs `Model`.
    pub fn from_model(model: mistralrs::Model) -> Self {
        Self {
            model: Arc::new(model),
        }
    }

    /// Build using `TextModelBuilder`.
    ///
    /// Standard text models from HuggingFace.
    pub async fn from_text(
        model_id: &str,
        isq: Option<mistralrs::IsqType>,
        chat_template: Option<&str>,
    ) -> anyhow::Result<Self> {
        let mut builder = mistralrs::TextModelBuilder::new(model_id)
            .with_logging()
            .from_hf_cache_pathf(cache_dir());
        if let Some(isq) = isq {
            builder = builder.with_isq(isq);
        }
        if let Some(template) = chat_template {
            builder = builder.with_chat_template(template);
        }
        let model = builder.build().await?;
        Ok(Self::from_model(model))
    }

    /// Build using `GgufModelBuilder`.
    ///
    /// GGUF quantized models from HuggingFace. The `model_id` is the HF repo
    /// ID; mistralrs auto-discovers GGUF files in the repo.
    pub async fn from_gguf(model_id: &str, chat_template: Option<&str>) -> anyhow::Result<Self> {
        // Pass empty files vec — mistralrs will auto-detect GGUF files.
        let mut builder =
            mistralrs::GgufModelBuilder::new(model_id, Vec::<String>::new()).with_logging();
        if let Some(template) = chat_template {
            builder = builder.with_chat_template(template);
        }
        let model = builder.build().await?;
        Ok(Self::from_model(model))
    }

    /// Access the inner mistralrs `Model`.
    pub fn model(&self) -> &mistralrs::Model {
        &self.model
    }

    /// Query the context length for a given model ID.
    ///
    /// Returns None if the model doesn't report a sequence length.
    pub fn context_length(&self, model: &str) -> Option<usize> {
        self.model
            .max_sequence_length_with_model(Some(model))
            .ok()
            .flatten()
    }

    /// Build using `VisionModelBuilder`.
    ///
    /// Vision-language models from HuggingFace.
    pub async fn from_vision(
        model_id: &str,
        isq: Option<mistralrs::IsqType>,
        chat_template: Option<&str>,
    ) -> anyhow::Result<Self> {
        let mut builder = mistralrs::VisionModelBuilder::new(model_id)
            .with_logging()
            .from_hf_cache_pathf(cache_dir());
        if let Some(isq) = isq {
            builder = builder.with_isq(isq);
        }
        if let Some(template) = chat_template {
            builder = builder.with_chat_template(template);
        }
        let model = builder.build().await?;
        Ok(Self::from_model(model))
    }
}

/// Walrus model cache directory: `~/.cache/walrus/models/`.
fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .expect("no platform cache directory")
        .join("walrus")
        .join("models")
}
