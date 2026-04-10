//! Node configuration.

pub use crate::mcp::{McpHandler, McpServerConfig};
pub use loader::{DEFAULT_CONFIG, scaffold_config_dir};
#[cfg(unix)]
pub use wcore::paths::SOCKET_PATH;
pub use wcore::{
    AgentConfig, ManifestConfig, NodeConfig, ProviderDef, ResolvedManifest, load_agents_dir,
    load_agents_dirs,
    paths::{AGENTS_DIR, CONFIG_DIR, CONFIG_FILE, SKILLS_DIR},
    resolve_manifests, validate_providers,
};

mod backfill;
mod loader;

pub use backfill::{backfill_local_agent_ids, migrate_local_agent_prompts};
