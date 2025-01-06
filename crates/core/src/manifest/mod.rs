//! Model manifest

use std::collections::HashMap;
pub use {
    family::{Family, Release},
    quant::Quantization,
};

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
}

impl Manifest {
    /// Create a new manifest from a model name
    pub fn new(name: &str) -> anyhow::Result<Self> {
        let release = Release::new(name)?;
        Ok(Self {
            name: name.into(),
            quantization: match release.family {
                Family::Llama => Quantization::Q4_0,
            },
            release,
            revision: [0; 12],
            params: HashMap::new(),
        })
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            name: "llama2".into(),
            release: Release::default(),
            quantization: Quantization::Q4_0,
            revision: [0; 12],
            params: HashMap::new(),
        }
    }
}
