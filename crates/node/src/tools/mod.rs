//! Tool handler factories for the node.
//!
//! Each submodule provides factory functions that return `(Tool, ToolHandler)`
//! pairs. The node builder registers them at startup — runtime has no
//! hardcoded tool dispatch.

pub mod ask_user;
pub mod delegate;
pub mod mcp;
pub mod memory;
pub mod os;
pub mod skill;
