//! LLM configuration

use candle_transformers::models::llama::LlamaConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Model to use.
    pub model: Model,
    /// Model revision.
    pub revision: String,
    /// Model configuration.
    pub config: ModelConfig,
    /// Use pytorch weights rather than the safetensors ones.
    pub pth: bool,
    /// Use CPU rather than GPU.
    pub cpu: bool,
}

/// Model configuration
#[derive(Debug, Default, Deserialize, Clone)]
pub enum Model {
    /// As known as all-MiniLM-L6-V2.
    #[default]
    MiniLm,
}

/// Model configuration
#[derive(Debug, Deserialize, Clone)]
pub enum ModelConfig {
    /// Llama model.
    Llama(LlamaConfig),
}
