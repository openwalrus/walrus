//! WHS (Walrus Hook Service) protocol types.
//!
//! Separate from the client protocol (`ClientMessage`/`ServerMessage`).
//! Only hook services speak WHS. Reuses the same wire codec
//! (`codec::read_message`/`write_message`).

use crate::model::Tool;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// Capabilities a hook service declares at handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Service provides these named tools.
    Tools(Vec<String>),
    /// Service implements `Query` for opaque service-specific queries.
    Query,
}

/// Messages sent by the daemon to a hook service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WhsRequest {
    /// Initiate handshake.
    Hello { version: String },
    /// Request tool schemas from the service.
    RegisterTools,
    /// Dispatch a tool call. Args is a JSON-encoded string.
    ToolCall {
        name: CompactString,
        args: String,
        agent: CompactString,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_id: Option<u64>,
    },
    /// Forward an opaque query to the service (routed by service name).
    ServiceQuery { query: String },
    /// Request graceful shutdown.
    Shutdown,
}

/// Messages sent by a hook service to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WhsResponse {
    /// Handshake response — declares service name and capabilities.
    Ready {
        version: String,
        service: CompactString,
        capabilities: Vec<Capability>,
    },
    /// Tool schema registration response.
    ToolSchemas { tools: Vec<Tool> },
    /// Tool call result.
    ToolResult { result: String },
    /// Query result.
    ServiceQueryResult { result: String },
    /// Error response.
    Error { message: String },
}
