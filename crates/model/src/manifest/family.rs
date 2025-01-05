use crate::manifest::Quantization;
use anyhow::Result;
use std::{fmt::Display, str::FromStr};

/// Release info of a model
#[derive(Debug)]
pub struct Release {
    /// The family of the model
    pub family: Family,

    /// The version of the model
    pub version: u8,

    /// The parameters of the model in billions
    pub parameters: f32,

    /// The tag of the model
    pub tag: Option<String>,
}

impl Release {
    /// Create a new release from a model name
    pub fn new(model: &str) -> Result<Self> {
        match model {
            "llama2" | "llama2-7b" | "llama2-7b-chat" => Ok(Self {
                family: Family::Llama,
                version: 2,
                parameters: 6.74,
                tag: Some("chat".into()),
            }),
            _ => anyhow::bail!("invalid model: {model}"),
        }
    }

    /// Get the repo of the model
    pub fn repo(&self) -> Result<&str> {
        match self.family.as_ref() {
            "llama" => Ok("TheBloke/Llama-2-7B-Chat-GGUF"),
            "gemma" => Ok("bartowski/gemma-2-2b-it-GGUF"),
            _ => anyhow::bail!("invalid family: {}", self.family),
        }
    }

    /// Get the model path of the model
    ///
    /// NOTE: only support llama2 for now
    pub fn model(&self, quant: Quantization) -> String {
        format!(
            "llama-2-{}b-{}.{}.gguf",
            self.parameters.ceil() as u8,
            self.tag.as_deref().unwrap_or("chat"),
            quant
        )
    }
}

impl Default for Release {
    fn default() -> Self {
        Self {
            family: Family::Llama,
            version: 2,
            parameters: 6.74,
            tag: Some("chat".into()),
        }
    }
}

impl Display for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}-{}b-{}",
            self.family.as_ref(),
            self.version,
            self.parameters.ceil() as u8,
            self.tag.as_deref().unwrap_or("chat")
        )
    }
}

/// The family of the model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Family {
    /// Llama from Meta
    #[default]
    Llama,
}

impl AsRef<str> for Family {
    fn as_ref(&self) -> &str {
        match self {
            Family::Llama => "llama",
        }
    }
}

impl Display for Family {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl FromStr for Family {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            _ => Family::Llama,
        })
    }
}

#[test]
fn test_fmt_release() {
    assert_eq!(Release::default().to_string(), "llama2-7b-chat");
    assert_eq!(
        Release::default().model(Quantization::Q4_0),
        "llama-2-7b-chat.Q4_0.gguf"
    );
}
