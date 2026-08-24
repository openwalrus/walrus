//! Agents, one file each.
//!
//! There is no name index. A name is found by reading the configs, which
//! for a store holding a handful of agents costs less than an index that
//! can disagree with them.

use crate::backend::{self, Backend};
use anyhow::Result;
use std::str::FromStr;
use store::{
    AgentConfig, AgentId,
    interface::{Agents, validate_table_name},
};

impl Backend {
    fn agent_path(&self, id: &AgentId) -> std::path::PathBuf {
        self.agents_dir()
            .join(format!("{}.json", backend::encode(&id.to_string())))
    }

    fn default_agent_path(&self) -> std::path::PathBuf {
        self.agents_dir().join("default")
    }

    /// Every stored config, in name order.
    async fn all_agents(&self) -> Result<Vec<AgentConfig>> {
        let mut out = Vec::new();
        for name in backend::names_in(&self.agents_dir(), ".json").await? {
            let Ok(id) = AgentId::from_str(&name) else {
                continue;
            };
            if let Some(config) = backend::read_json(&self.agent_path(&id)).await? {
                out.push(config);
            }
        }
        out.sort_by(|a: &AgentConfig, b: &AgentConfig| a.name.cmp(&b.name));
        Ok(out)
    }
}

impl Agents for Backend {
    async fn load_agent(&self, id: &AgentId) -> Result<Option<AgentConfig>> {
        backend::read_json(&self.agent_path(id)).await
    }

    async fn load_agent_by_name(&self, name: &str) -> Result<Option<AgentConfig>> {
        Ok(self
            .all_agents()
            .await?
            .into_iter()
            .find(|c| c.name == name))
    }

    async fn agent_ids(&self) -> Result<Vec<AgentId>> {
        Ok(self.all_agents().await?.into_iter().map(|c| c.id).collect())
    }

    async fn upsert_agent(&self, config: &AgentConfig) -> Result<()> {
        validate_table_name("agent", &config.name)?;
        backend::write_json(&self.agent_path(&config.id), config).await
    }

    async fn delete_agent(&self, id: &AgentId) -> Result<bool> {
        Ok(tokio::fs::remove_file(self.agent_path(id)).await.is_ok())
    }

    async fn rename_agent(&self, id: &AgentId, new_name: &str) -> Result<bool> {
        validate_table_name("agent", new_name)?;
        let Some(mut config) = self.load_agent(id).await? else {
            return Ok(false);
        };
        config.name = new_name.to_owned();
        backend::write_json(&self.agent_path(id), &config).await?;
        Ok(true)
    }

    async fn default_agent(&self) -> Result<Option<AgentId>> {
        let Ok(raw) = tokio::fs::read_to_string(self.default_agent_path()).await else {
            return Ok(None);
        };
        Ok(AgentId::from_str(raw.trim()).ok())
    }

    async fn set_default_agent(&self, id: &AgentId) -> Result<()> {
        Ok(tokio::fs::write(self.default_agent_path(), id.to_string()).await?)
    }
}
