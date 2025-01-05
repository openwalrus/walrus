//! LLM configuration

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Model revision.
    pub revision: String,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            revision: "main".to_string(),
            pth: false,
            cpu: false,
            seed: 1_024_243_212,
            temp: Some(0.6),
            top_p: Some(0.9),
            top_k: Some(50),
            sample_len: 256,
            repeat_penalty: 1.0,
            repeat_last_n: 64,
        }
    }
}
