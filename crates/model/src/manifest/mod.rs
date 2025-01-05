//! Model manifest

use std::collections::HashMap;
pub use {family::Release, quant::Quantization};

mod family;
mod quant;

/// Manifest of a quantized model
#[derive(Debug)]
pub struct Manifest {
    /// The name of the model
    pub name: String,

    /// The release of the model
    pub release: Release,

    /// The K-quantization of the model
    pub quantization: Quantization,

    /// The revision of the model
    pub revision: [u8; 12],

    /// The parameters of the model
    pub params: HashMap<String, String>,

    /// The license of the model
    pub license: String,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            name: "llama2".into(),
            release: Release::default(),
            quantization: Quantization::Q4_0,
            revision: [0; 12],
            params: HashMap::new(),
            license: "llama2".into(),
        }
    }
}
