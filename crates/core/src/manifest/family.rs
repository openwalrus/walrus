//! Model family

use anyhow::Result;
pub use llama::LlamaVer;
use std::{fmt::Display, str::FromStr};

/// The family of the model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// Llama from Meta
    Llama {
        /// The version of the model
        version: LlamaVer,

        /// The parameters of the model
        params: Params,

        /// The tag of the model
        tag: Tag,
    },
}

impl Default for Family {
    fn default() -> Self {
        Self::Llama {
            version: LlamaVer::V3_2,
            params: Params::V1B,
            tag: Tag::Instruct,
        }
    }
}

impl From<&str> for Family {
    fn from(s: &str) -> Self {
        Self::from_str(s).unwrap_or_default()
    }
}

impl Display for Family {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Family::Llama {
                    version,
                    params,
                    tag,
                } => format!("Llama-{version}-{params}-{tag}"),
            }
        )
    }
}

impl FromStr for Family {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let model = s
            .trim()
            .to_lowercase()
            .replace('-', "")
            .replace("instruct", "");

        match model.as_ref() {
            "llama3.18b" => Ok(Family::Llama {
                version: LlamaVer::V3_1,
                params: Params::V8B,
                tag: Tag::Instruct,
            }),
            "llama3.21b" => Ok(Family::Llama {
                version: LlamaVer::V3_2,
                params: Params::V1B,
                tag: Tag::Instruct,
            }),
            "llama3.23b" => Ok(Family::Llama {
                version: LlamaVer::V3_2,
                params: Params::V3B,
                tag: Tag::Instruct,
            }),
            _ => {
                tracing::warn!("invalid family {s}, using default llama-3.2-1B-Instruct");
                Ok(Family::Llama {
                    version: LlamaVer::V3_2,
                    params: Params::V1B,
                    tag: Tag::Instruct,
                })
            }
        }
    }
}

/// The parameters of the model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Params {
    V1B,
    V3B,
    V8B,
}

impl Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Params::V1B => "1B",
                Params::V3B => "3B",
                Params::V8B => "8B",
            }
        )
    }
}

/// The tag of the model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tag {
    #[default]
    Instruct,
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

mod llama {
    use std::fmt::Display;

    /// The version of the llama model
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub enum LlamaVer {
        V3_1,
        #[default]
        V3_2,
    }

    impl Display for LlamaVer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    LlamaVer::V3_1 => "3.1",
                    LlamaVer::V3_2 => "3.2",
                }
            )
        }
    }
}
