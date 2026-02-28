//! Local LLM provider via mistralrs.
//!
//! Wraps `mistralrs::Model` for native on-device inference (DD#59).
//! No HTTP transport — inference runs in-process.

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

    /// Build from a HuggingFace model ID using `TextModelBuilder`.
    ///
    /// Optionally applies in-situ quantization via `isq`.
    pub async fn from_hf(
        model_id: &str,
        isq: Option<mistralrs::IsqType>,
    ) -> anyhow::Result<Self> {
        let mut builder = mistralrs::TextModelBuilder::new(model_id).with_logging();
        if let Some(isq) = isq {
            builder = builder.with_isq(isq);
        }
        let model = builder.build().await?;
        Ok(Self::from_model(model))
    }

    /// Build from local GGUF files using `GgufModelBuilder`.
    pub async fn from_gguf(
        model_id: &str,
        files: Vec<String>,
        chat_template: Option<&str>,
    ) -> anyhow::Result<Self> {
        let mut builder = mistralrs::GgufModelBuilder::new(model_id, files).with_logging();
        if let Some(template) = chat_template {
            builder = builder.with_chat_template(template);
        }
        let model = builder.build().await?;
        Ok(Self::from_model(model))
    }
}
