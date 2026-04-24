//! Daemon-level configuration mutations: provider / model / MCP / skill.
//! Pure storage-backed queries (list_providers, list_models, active_model,
//! provider_name_for_model) live on `Runtime<C>` directly.

use crate::daemon::Daemon;
use anyhow::{Context, Result};
use crabllm_core::Provider;
use wcore::protocol::message::*;
use wcore::storage::Storage;

impl<P: Provider + 'static> Daemon<P> {
    pub(crate) async fn set_provider(&self, name: String, config: String) -> Result<ProviderInfo> {
        let def: wcore::ProviderDef =
            serde_json::from_str(&config).context("invalid ProviderDef JSON")?;
        let rt = self.runtime.read().await.clone();
        let storage = rt.storage();
        let mut node_config = storage.load_config()?;
        node_config.provider.insert(name.clone(), def);
        wcore::validate_providers(&node_config.provider)?;
        storage.save_config(&node_config)?;
        self.reload().await?;

        let rt = self.runtime.read().await.clone();
        list_providers(&rt)?
            .into_iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("provider '{name}' missing after configure"))
    }

    pub(crate) async fn delete_provider(&self, name: &str) -> Result<()> {
        let rt = self.runtime.read().await.clone();
        let storage = rt.storage();
        let mut config = storage.load_config()?;
        if config.provider.remove(name).is_none() {
            anyhow::bail!("provider '{name}' not found");
        }
        storage.save_config(&config)?;
        self.reload().await
    }

    pub(crate) async fn set_active_model(&self, model: String) -> Result<()> {
        let rt = self.runtime.read().await.clone();
        let storage = rt.storage();

        let config = storage.load_config()?;
        let model_exists = config
            .provider
            .values()
            .any(|def| def.models.iter().any(|m| m == &model));
        if !model_exists {
            anyhow::bail!("model '{model}' not found in any provider");
        }

        let mut crab = storage
            .load_agent_by_name(wcore::paths::DEFAULT_AGENT)?
            .unwrap_or_else(|| crate::storage::default_crab(&model));
        let prompt = std::mem::take(&mut crab.system_prompt);
        crab.model = model;
        storage.upsert_agent(&crab, &prompt)?;
        self.reload().await
    }

    pub(crate) async fn list_mcps(&self) -> Result<Vec<McpInfo>> {
        let connected: std::collections::BTreeMap<String, usize> = self
            .mcp
            .cached_list()
            .into_iter()
            .map(|(name, tools)| (name, tools.len()))
            .collect();

        let mut mcps = Vec::new();

        // Storage wins over manifest on name conflict.
        let storage_mcps = {
            let rt = self.runtime.read().await.clone();
            rt.storage().list_mcps()?
        };
        for (name, cfg) in &storage_mcps {
            let (status, tool_count) = mcp_status(&connected, name);
            mcps.push(mcp_to_info(
                name,
                cfg,
                "local",
                SourceKind::Local,
                status,
                tool_count,
            ));
        }

        for (plugin_name, plugin_manifest) in super::plugin::scan_plugin_manifests(&self.config_dir)
        {
            for (name, mcp_res) in &plugin_manifest.mcps {
                if mcps.iter().any(|m| m.name == *name) {
                    continue;
                }
                let (status, tool_count) = mcp_status(&connected, name);
                let cfg = mcp_res.to_server_config();
                mcps.push(mcp_to_info(
                    name,
                    &cfg,
                    &plugin_name,
                    SourceKind::Plugin,
                    status,
                    tool_count,
                ));
            }
        }

        Ok(mcps)
    }

    pub(crate) async fn upsert_mcp(&self, config_json: String) -> Result<McpInfo> {
        let cfg: wcore::McpServerConfig =
            serde_json::from_str(&config_json).context("invalid McpServerConfig JSON")?;
        let name = cfg.name.clone();
        {
            let rt = self.runtime.read().await.clone();
            rt.storage().upsert_mcp(&cfg)?;
        }
        self.reload().await?;

        // Re-list to surface the runtime status (connected/failed/etc).
        let mcps = self.list_mcps().await?;
        mcps.into_iter()
            .find(|m| m.name == name)
            .ok_or_else(|| anyhow::anyhow!("mcp '{name}' missing from listing after upsert"))
    }

    pub(crate) async fn delete_mcp(&self, name: &str) -> Result<bool> {
        let removed = {
            let rt = self.runtime.read().await.clone();
            rt.storage().delete_mcp(name)?
        };
        if removed {
            self.reload().await?;
        }
        Ok(removed)
    }

    pub(crate) fn list_skills(&self) -> Vec<SkillInfo> {
        let dirs = wcore::resolve_dirs(&self.config_dir);
        let local_skills_dir = self.config_dir.join(wcore::paths::SKILLS_DIR);

        let dir_to_pkg: std::collections::BTreeMap<_, _> = dirs
            .plugin_skill_dirs
            .iter()
            .map(|(id, dir)| (dir.clone(), id.clone()))
            .collect();

        let mut seen = std::collections::BTreeSet::new();
        let mut skills = Vec::new();

        for dir in &dirs.skill_dirs {
            let (source, source_kind) = if *dir == local_skills_dir {
                ("local".to_string(), SourceKind::Local)
            } else if let Some(pkg_id) = dir_to_pkg.get(dir) {
                (pkg_id.clone(), SourceKind::Plugin)
            } else {
                let name = wcore::external_source_name(dir).unwrap_or("external");
                (name.to_string(), SourceKind::External)
            };

            for name in wcore::scan_skill_names(dir) {
                if !seen.insert(name.clone()) {
                    continue;
                }
                skills.push(SkillInfo {
                    name,
                    source: source.clone(),
                    source_kind: source_kind.into(),
                });
            }
        }
        skills
    }
}

/// Build the protocol-facing provider list. Lives here (not on Runtime)
/// because the `ProviderInfo.config` field carries the `ProviderDef`
/// serialized as JSON — a wire-format concern, not a runtime concern.
pub(super) fn list_providers<C: runtime::Config>(
    rt: &runtime::Runtime<C>,
) -> Result<Vec<ProviderInfo>> {
    let config = rt.storage().load_config()?;
    let active_model = rt.active_model();
    Ok(config
        .provider
        .iter()
        .map(|(name, def)| ProviderInfo {
            name: name.clone(),
            active: !active_model.is_empty() && def.models.contains(&active_model),
            config: serde_json::to_string(def).unwrap_or_default(),
        })
        .collect())
}

pub(super) fn provider_presets() -> Vec<ProviderPresetInfo> {
    wcore::config::PROVIDER_PRESETS
        .iter()
        .map(|p| ProviderPresetInfo {
            name: p.name.to_string(),
            kind: ProviderKind::from(&p.kind).into(),
            base_url: p.base_url.to_string(),
            fixed_base_url: p.fixed_base_url.to_string(),
            default_model: p.default_model.to_string(),
        })
        .collect()
}

fn mcp_status(
    connected: &std::collections::BTreeMap<String, usize>,
    name: &str,
) -> (McpStatus, u32) {
    match connected.get(name) {
        Some(&count) => (McpStatus::Connected, count as u32),
        None => (McpStatus::Failed, 0),
    }
}

fn mcp_to_info(
    name: &str,
    cfg: &wcore::McpServerConfig,
    source: &str,
    source_kind: SourceKind,
    status: McpStatus,
    tool_count: u32,
) -> McpInfo {
    McpInfo {
        name: name.to_string(),
        command: cfg.command.clone(),
        args: cfg.args.clone(),
        env: cfg
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        url: cfg.url.clone().unwrap_or_default(),
        auth: cfg.auth,
        source: source.to_string(),
        auto_restart: cfg.auto_restart,
        source_kind: source_kind.into(),
        status: status.into(),
        error: String::new(),
        tool_count,
    }
}
