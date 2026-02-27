//! Runtime builder — constructs a fully-configured Runtime from GatewayConfig.

use crate::MemoryBackend;
use crate::config::{self, MemoryBackendKind, ProviderKind};
use crate::gateway::GatewayHook;
use crate::provider::Provider;
use anyhow::Result;
use claude::Claude;
use deepseek::DeepSeek;
use llm::LLM;
use openai::OpenAI;
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
    // Construct memory backend.
    let memory = match config.memory.backend {
        MemoryBackendKind::InMemory => {
            tracing::info!("using in-memory backend");
            MemoryBackend::in_memory()
        }
        MemoryBackendKind::Sqlite => {
            let data_dir = config_dir.join(config::DATA_DIR);
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join(config::MEMORY_DB);
            let path = db_path.to_str().expect("non-UTF-8 config path");
            tracing::info!("using sqlite backend at {path}");
            MemoryBackend::sqlite(path)?
        }
    };

    // Construct provider.
    let client = llm::Client::new();
    let key = &config.llm.api_key;
    let provider = match config.llm.provider {
        ProviderKind::DeepSeek => match &config.llm.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::DeepSeek(DeepSeek::new(client, key)?),
        },
        ProviderKind::OpenAI => match &config.llm.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::OpenAI(OpenAI::api(client, key)?),
        },
        ProviderKind::Grok => match &config.llm.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::OpenAI(OpenAI::grok(client, key)?),
        },
        ProviderKind::Qwen => match &config.llm.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::OpenAI(OpenAI::qwen(client, key)?),
        },
        ProviderKind::Kimi => match &config.llm.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::OpenAI(OpenAI::kimi(client, key)?),
        },
        ProviderKind::Ollama => match &config.llm.base_url {
            Some(url) => Provider::OpenAI(OpenAI::custom(client, key, url)?),
            None => Provider::OpenAI(OpenAI::ollama(client)?),
        },
        ProviderKind::Claude => match &config.llm.base_url {
            Some(url) => Provider::Claude(Claude::custom(client, key, url)?),
            None => Provider::Claude(Claude::anthropic(client, key)?),
        },
    };
    tracing::info!(
        "provider {:?} initialized for model {}",
        config.llm.provider,
        config.llm.model
    );

    // Build general config.
    let general = General {
        model: config.llm.model.clone(),
        ..General::default()
    };

    // Build runtime.
    let mut runtime = Runtime::<GatewayHook>::new(general, provider, memory);

    // Load agents from markdown files.
    let agents = runtime::load_agents_dir(&config_dir.join(config::AGENTS_DIR))?;
    for agent in agents {
        tracing::info!("registered agent '{}'", agent.name);
        runtime.add_agent(agent);
    }

    // Load skills if directory exists.
    let skills_dir = config_dir.join(config::SKILLS_DIR);
    match SkillRegistry::load_dir(&skills_dir, agent::SkillTier::Workspace) {
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
