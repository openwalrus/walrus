//! Shared configuration types used across crates.

pub mod daemon;
pub mod hooks;
pub mod manifest;
pub mod mcp;
pub mod provider;
pub mod system;

pub use daemon::{DaemonConfig, validate_providers};
pub use hooks::{BashConfig, HooksConfig, MemoryConfig};
pub use manifest::{
    PackageMeta, ResolvedDirs, Setup, check_skill_conflicts, external_source_name, load_agents_dir,
    load_agents_dirs, repo_slug, resolve_dirs, scan_skill_names,
};
pub use mcp::McpServerConfig;
pub use provider::{ApiStandard, PROVIDER_PRESETS, ProviderDef, ProviderPreset};
pub use system::TasksConfig;
