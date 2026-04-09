//! Hook module — re-exports Env<DaemonHost, FsStorage> as DaemonEnv.

pub mod host;

/// The daemon's environment type — Env with DaemonHost for
/// server-specific dispatch and FsStorage for persistence.
pub type DaemonEnv = runtime::Env<host::DaemonHost, crate::storage::FsStorage>;
