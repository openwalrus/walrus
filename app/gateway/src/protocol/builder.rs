//! Runtime builder — constructs a fully-configured Runtime from GatewayConfig.

use crate::MemoryBackend;
use crate::config::MemoryBackendKind;
use crate::protocol::GatewayHook;
use anyhow::Result;
use deepseek::DeepSeek;
use llm::LLM;
use runtime::{General, McpBridge, Runtime, SkillRegistry};

/// Build a fully-configured `Runtime<GatewayHook>` from a `GatewayConfig`.
///
/// Constructs memory backend, LLM provider, skills registry, MCP bridges,
/// and registers all agents from config. Async because MCP server connection
/// requires spawning child processes.
pub async fn build_runtime(config: &crate::GatewayConfig) -> Result<Runtime<GatewayHook>> {
    // Construct memory backend.
    let memory = match config.memory.backend {
        MemoryBackendKind::InMemory => {
            tracing::info!("using in-memory backend");
            MemoryBackend::in_memory()
        }
        MemoryBackendKind::Sqlite => {
            let path = config.memory.path.as_deref().unwrap_or("walrus-gateway.db");
            tracing::info!("using sqlite backend at {path}");
            MemoryBackend::sqlite(path)?
        }
    };

    // Construct provider.
    let provider = DeepSeek::new(llm::Client::new(), &config.llm.api_key)?;
    tracing::info!("provider initialized for model {}", config.llm.model);

    // Build general config.
    let general = General {
        model: config.llm.model.clone(),
        ..General::default()
    };

    // Build runtime.
    let mut runtime = Runtime::<GatewayHook>::new(general, provider, memory);

    // Load skills if configured.
    if let Some(ref skills_config) = config.skills {
        let registry =
            SkillRegistry::load_dir(&skills_config.directory, agent::SkillTier::Workspace)?;
        runtime.set_skills(registry);
        tracing::info!("loaded skills from {}", skills_config.directory);
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

    // Register agents from config.
    for agent_config in &config.agents {
        let agent = agent::Agent {
            name: agent_config.name.clone(),
            description: agent_config.description.clone(),
            system_prompt: agent_config.system_prompt.clone(),
            tools: agent_config.tools.clone(),
            skill_tags: agent_config.skill_tags.clone(),
        };
        tracing::info!("registered agent '{}'", agent.name);
        runtime.add_agent(agent);
    }

    Ok(runtime)
}
