//! WHS (Walrus Hook Service) protocol types.
//!
//! Now generated from proto/protocol.proto. Re-exported here
//! for backward-compatible import paths.

pub use crate::protocol::proto::{
    BeforeRunCap, BuildAgentCap, Capability, CompactCap, QueryCap, SimpleMessage, ToolDef,
    ToolsList, WhsBeforeRun, WhsBeforeRunResult, WhsBuildAgent, WhsBuildAgentResult, WhsCompact,
    WhsCompactResult, WhsConfigure, WhsConfigured, WhsError, WhsHello, WhsReady, WhsRegisterTools,
    WhsRequest, WhsResponse, WhsServiceQuery, WhsServiceQueryResult, WhsShutdown, WhsToolCall,
    WhsToolResult, WhsToolSchemas, capability, whs_request, whs_response,
};
