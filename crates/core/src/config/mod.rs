//! Shared configuration types used across crates.

pub mod manifest;
pub mod mcp;
pub mod provider;

pub use manifest::{
    ManifestConfig, PackageMeta, ResolvedManifest, Setup, check_skill_conflicts, load_agents_dir,
    load_agents_dirs, repo_slug, resolve_manifests,
};
pub use mcp::McpServerConfig;
pub use provider::{ApiStandard, ProviderDef};
