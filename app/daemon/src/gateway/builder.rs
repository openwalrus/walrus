//! Runtime builder — constructs a fully-configured Runtime from GatewayConfig.

use crate::MemoryBackend;
use crate::config;
use crate::gateway::GatewayHook;
use anyhow::Result;
use provider::ProviderManager;
use runtime::{General, McpBridge, Runtime, SkillRegistry};
use std::path::Path;

/// Build a fully-configured `Runtime<GatewayHook>` from config and directory.
///
/// Loads agents from `config_dir/agents/*.md`, skills from `config_dir/skills/`,
/// memory from `config_dir/data/memory.db` (when sqlite), and MCP servers
/// from TOML config.
pub async fn build_runtime(
    config: &crate::GatewayConfig,
    config_dir: &Path,
) -> Result<Runtime<GatewayHook>> {
    // Construct in-memory backend.
    let memory = MemoryBackend::in_memory();
    tracing::info!("using in-memory backend");

    // Construct provider manager from config list.
    let manager = ProviderManager::from_configs(&config.models).await?;
    tracing::info!(
        "provider manager initialized — active model: {}",
        manager.active_model()
    );

    // Build general config.
    let general = General {
        model: manager.active_model(),
        ..General::default()
    };

    // Build runtime.
    let mut runtime = Runtime::<GatewayHook>::new(general, manager, memory);

    // Load agents from markdown files.
    let agents = runtime::load_agents_dir(&config_dir.join(config::AGENTS_DIR))?;
    for agent in agents {
        tracing::info!("registered agent '{}'", agent.name);
        runtime.add_agent(agent);
    }

    // Load skills if directory exists.
    let skills_dir = config_dir.join(config::SKILLS_DIR);
    match SkillRegistry::load_dir(&skills_dir, wcore::SkillTier::Workspace) {
        Ok(registry) => {
            tracing::info!("loaded {} skill(s)", registry.len());
            runtime.set_skills(registry);
        }
        Err(e) => {
            tracing::warn!("could not load skills from {}: {e}", skills_dir.display());
        }
    }

    // Connect MCP servers if configured.
    if !config.mcp_servers.is_empty() {
        let bridge = McpBridge::new();
        for server_config in &config.mcp_servers {
            let mut cmd = tokio::process::Command::new(&server_config.command);
            cmd.args(&server_config.args);
            for (k, v) in &server_config.env {
                cmd.env(k, v);
            }
            if let Err(e) = bridge.connect_stdio(cmd).await {
                tracing::warn!("failed to connect MCP server '{}': {e}", server_config.name);
            } else {
                tracing::info!("connected MCP server '{}'", server_config.name);
            }
        }
        runtime.connect_mcp(bridge);
        if let Err(e) = runtime.register_mcp_tools().await {
            tracing::warn!("failed to register MCP tools: {e}");
        }
    }

    Ok(runtime)
}
