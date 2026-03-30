//! Hook module — re-exports Env<DaemonHost> as DaemonEnv.

pub mod host;

/// The daemon's environment type — Env with DaemonHost for server-specific dispatch.
pub type DaemonEnv = runtime::Env<host::DaemonHost>;
