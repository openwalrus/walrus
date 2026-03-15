//! WHS (Walrus Hook Service) protocol types.
//!
//! Generated from `proto/whs.proto`. Re-exported here for stable
//! `wcore::protocol::whs::*` import paths.

pub use crate::protocol::proto::whs_proto::{
    AfterRunCap, BeforeRunCap, BuildAgentCap, Capability, CompactCap, EventObserverCap, InferCap,
    QueryCap, SimpleMessage, ToolDef, ToolsList, WhsAfterRun, WhsAfterRunResult, WhsBeforeRun,
    WhsBeforeRunResult, WhsBuildAgent, WhsBuildAgentResult, WhsCompact, WhsCompactResult,
    WhsConfigure, WhsConfigured, WhsError, WhsEvent, WhsGetSchema, WhsHello, WhsInferRequest,
    WhsInferResult, WhsReady, WhsRegisterTools, WhsRequest, WhsResponse, WhsSchemaResult,
    WhsServiceQuery, WhsServiceQueryResult, WhsShutdown, WhsToolCall, WhsToolResult,
    WhsToolSchemas, capability, whs_request, whs_response,
};
