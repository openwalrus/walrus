//! MCP (Model Context Protocol) client, bridge, and dispatcher.
//!
//! Three layers:
//! - [`client`] — minimal JSON-RPC 2.0 client over stdio or HTTP
//! - [`bridge`] — fleet of connected peers, tool cache, call routing
//! - [`handler`] / [`dispatch`] — config-driven load, port-file discovery,
//!   meta-tool dispatch

pub use {bridge::McpBridge, handler::McpHandler};

pub mod bridge;
pub mod client;
pub mod dispatch;
pub mod handler;
