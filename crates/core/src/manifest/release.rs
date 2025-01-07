//! Model release

use crate::manifest::{Family, LlamaVer, Params, Quantization};
use std::fmt::Display;

/// Release info of a model
#[derive(Debug, Default)]
pub struct Release {
    /// The family of the model
    pub family: Family,

    /// The quantization of the model
    pub quant: Quantization,
}

impl Release {
    /// Create a new release from a model name
    pub fn new(model: &str) -> Self {
        let family = Family::from(model);
        Self {
            family,
            quant: Quantization::Q4_K_M,
        }
    }

    /// Get the repo of the model
    pub fn repo(&self) -> &str {
        let Family::Llama {
            version, params, ..
        } = self.family;

        match (version, params) {
            (LlamaVer::V3_1, _) => "MaziyarPanahi/Meta-Llama-3.1-8B-Instruct-GGUF",
            (LlamaVer::V3_2, Params::V3B) => "MaziyarPanahi/Llama-3.2-3B-Instruct-GGUF",
            _ => "MaziyarPanahi/Llama-3.2-1B-Instruct-GGUF",
        }
    }

    /// Get the tokenizer path from the tokenizer repo
    pub fn tokenizer(&self) -> &str {
        "llama3/tokenizer.json"
    }

    /// Get the model path of the model
    pub fn model(&self) -> String {
        format!("{}.gguf", self)
    }
}

impl Display for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.family, self.quant)
    }
}

#[test]
fn test_fmt_release() {
    assert_eq!(Release::default().to_string(), "llama2-7b-chat");
    assert_eq!(Release::default().model(), "llama-2-7b-chat.Q4_0.gguf");
}
