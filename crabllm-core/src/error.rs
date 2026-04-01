use serde::{Deserialize, Serialize};
use std::fmt;

/// Shared error type for the crabllm workspace.
#[derive(Debug)]
pub enum Error {
    /// TOML config parse error or missing env var.
    Config(String),
    /// Upstream provider returned an error status.
    Provider { status: u16, body: String },
    /// JSON serialization/deserialization error.
    Json(serde_json::Error),
    /// Catch-all for internal errors.
    Internal(String),
    /// Request to upstream provider timed out.
    Timeout,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Config(msg) => write!(f, "config error: {msg}"),
            Error::Provider { status, body } => {
                write!(f, "provider error (HTTP {status}): {body}")
            }
            Error::Json(e) => write!(f, "json error: {e}"),
            Error::Internal(msg) => write!(f, "internal error: {msg}"),
            Error::Timeout => write!(f, "request timed out"),
        }
    }
}

impl Error {
    /// Whether this error is transient and the request should be retried.
    /// Transient: 429 (rate limit), 500, 502, 503, 504 (server errors),
    /// and connection/internal errors.
    pub fn is_transient(&self) -> bool {
        match self {
            Error::Provider { status, .. } => matches!(status, 429 | 500 | 502 | 503 | 504),
            Error::Internal(_) | Error::Timeout => true,
            _ => false,
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Json(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "gateway")]
impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::Config(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

/// OpenAI-compatible error response returned to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: ApiErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub message: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ApiError {
    pub fn new(message: impl Into<String>, kind: impl Into<String>) -> Self {
        ApiError {
            error: ApiErrorBody {
                message: message.into(),
                kind: kind.into(),
                param: None,
                code: None,
            },
        }
    }
}
