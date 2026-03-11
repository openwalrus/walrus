//! Web search and fetch tools for agents.
//!
//! Registers `web_search` and `web_fetch` tool schemas. Dispatch methods
//! live on [`DaemonHook`](crate::hook::DaemonHook).

pub(crate) mod tool;
