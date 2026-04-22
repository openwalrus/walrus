//! Agent CRUD: list, get, create, update, delete.

use crate::daemon::Daemon;
use anyhow::{Context, Result};
use crabllm_core::Provider;
use wcore::protocol::message::*;
use wcore::storage::Storage;

pub(super) async fn list<P: Provider + 'static>(node: &Daemon<P>) -> Result<Vec<AgentInfo>> {
    let rt = node.runtime.read().await.clone();
    Ok(rt
        .agents()
        .into_iter()
        .map(|c| agent_config_to_info(&c))
        .collect())
}

pub(super) async fn get<P: Provider + 'static>(
    node: &Daemon<P>,
    name: String,
) -> Result<AgentInfo> {
    let rt = node.runtime.read().await.clone();
    let config = rt
        .agent(&name)
        .ok_or_else(|| anyhow::anyhow!("agent '{name}' not found"))?;
    Ok(agent_config_to_info(&config))
}

pub(super) async fn create<P: Provider + 'static>(
    node: &Daemon<P>,
    req: CreateAgentMsg,
) -> Result<AgentInfo> {
    validate_agent_name(&req.name)?;
    let mut config: wcore::AgentConfig =
        serde_json::from_str(&req.config).context("invalid AgentConfig JSON")?;
    if config.id.is_nil() {
        config.id = wcore::AgentId::new();
    }
    config.name = req.name.clone();

    let rt = node.runtime.read().await.clone();
    let storage = rt.storage();
    if storage.load_agent_by_name(&req.name)?.is_some() {
        anyhow::bail!("agent '{}' already exists", req.name);
    }
    storage.upsert_agent(&config, &req.prompt)?;

    register_agent_from_disk(node, &req.name).await?;
    get(node, req.name).await
}

pub(super) async fn update<P: Provider + 'static>(
    node: &Daemon<P>,
    req: UpdateAgentMsg,
) -> Result<AgentInfo> {
    validate_agent_name(&req.name)?;
    let mut config: wcore::AgentConfig =
        serde_json::from_str(&req.config).context("invalid AgentConfig JSON")?;
    let rt = node.runtime.read().await.clone();
    let storage = rt.storage();
    let existing = storage.load_agent_by_name(&req.name)?;
    if let Some(prev) = &existing {
        if config.id.is_nil() {
            config.id = prev.id;
        }
    } else if config.id.is_nil() {
        config.id = wcore::AgentId::new();
    }
    config.name = req.name.clone();
    let prompt = if req.prompt.is_empty() {
        existing.map(|a| a.system_prompt).unwrap_or_default()
    } else {
        req.prompt.clone()
    };
    storage.upsert_agent(&config, &prompt)?;
    register_agent_from_disk(node, &req.name).await?;
    get(node, req.name).await
}

pub(super) async fn rename<P: Provider + 'static>(
    node: &Daemon<P>,
    old_name: String,
    new_name: String,
) -> Result<AgentInfo> {
    validate_agent_name(&new_name)?;
    anyhow::ensure!(
        old_name != wcore::paths::DEFAULT_AGENT,
        "cannot rename the default agent '{old_name}'"
    );
    if old_name == new_name {
        return get(node, old_name).await;
    }

    let rt = node.runtime.read().await.clone();
    let storage = rt.storage();
    let existing = storage
        .load_agent_by_name(&old_name)?
        .ok_or_else(|| anyhow::anyhow!("agent '{old_name}' not found"))?;
    storage.rename_agent(&existing.id, &new_name)?;

    rt.remove_agent(&old_name);
    node.hook.unregister_scope(&old_name);

    register_agent_from_disk(node, &new_name).await?;
    get(node, new_name).await
}

pub(super) async fn delete<P: Provider + 'static>(node: &Daemon<P>, name: String) -> Result<bool> {
    let rt = node.runtime.read().await.clone();
    let storage = rt.storage();
    let Some(existing) = storage.load_agent_by_name(&name)? else {
        return Ok(false);
    };
    let removed = storage.delete_agent(&existing.id)?;
    if removed {
        rt.remove_agent(&name);
        node.hook.unregister_scope(&name);
    }
    Ok(removed)
}

async fn register_agent_from_disk<P: Provider + 'static>(
    node: &Daemon<P>,
    name: &str,
) -> Result<()> {
    let rt = node.runtime.read().await.clone();
    let agent_config = rt
        .storage()
        .load_agent_by_name(name)?
        .ok_or_else(|| anyhow::anyhow!("agent '{name}' missing from storage after upsert"))?;
    let registered = rt.upsert_agent(agent_config);
    node.hook.register_scope(name.to_owned(), &registered);
    Ok(())
}

fn validate_agent_name(name: &str) -> Result<()> {
    anyhow::ensure!(!name.is_empty(), "agent name cannot be empty");
    anyhow::ensure!(
        !name.contains('/') && !name.contains('\\') && !name.contains(".."),
        "agent name '{name}' contains invalid characters"
    );
    Ok(())
}

fn agent_config_to_info(config: &wcore::AgentConfig) -> AgentInfo {
    let json = serde_json::to_string(config).unwrap_or_default();
    AgentInfo {
        name: config.name.clone(),
        description: config.description.clone(),
        config: json,
        model: config.model.clone(),
        max_iterations: config.max_iterations as u32,
        thinking: config.thinking,
        members: config.members.clone(),
        skills: config.skills.clone(),
        mcps: config.mcps.clone(),
        compact_threshold: config.compact_threshold.map(|t| t as u32),
        compact_tool_max_len: config.compact_tool_max_len as u32,
    }
}
