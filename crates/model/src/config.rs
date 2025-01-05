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

    /// The seed of the model
    pub seed: u64,

    /// The temperature of the model
    pub temp: Option<f64>,

    /// The top-p of the model
    pub top_p: Option<f64>,

    /// The top-k of the model
    pub top_k: Option<usize>,

    /// The sample length of the model
    pub sample_len: usize,

    /// The repeat penalty of the model
    pub repeat_penalty: f32,

    /// The repeat last n of the model
    pub repeat_last_n: usize,
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
