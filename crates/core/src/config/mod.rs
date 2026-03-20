//! Shared configuration types used across crates.

pub mod command;
pub mod mcp;
pub mod provider;

pub use command::CommandConfig;
pub use mcp::McpServerConfig;
pub use provider::{ApiStandard, ProviderDef};
