//! Storage-backed configuration queries — providers, models, active agent.
//!
//! Only runtime-shaped data lives here. Wire-format conversions (e.g.
//! JSON-ifying `ProviderDef` into a string field) belong in the protocol
//! layer — runtime returns typed Rust values.

use super::Runtime;
use crate::Config;
use anyhow::Result;
use wcore::{
    paths,
    protocol::message::{ModelInfo, ProviderKind},
    storage::Storage,
};

impl<C: Config> Runtime<C> {
    /// The active model — defined as the default agent's `model` field.
    /// Empty string if the default agent is missing (pre-scaffold).
    pub fn active_model(&self) -> String {
        self.storage()
            .load_agent_by_name(paths::DEFAULT_AGENT)
            .ok()
            .flatten()
            .map(|c| c.model)
            .unwrap_or_default()
    }

    /// Return the provider name that owns the given model, or empty string
    /// if no provider declares it.
    pub fn provider_name_for_model(&self, model: &str) -> String {
        self.storage()
            .load_config()
            .ok()
            .and_then(|c| {
                c.provider
                    .iter()
                    .find(|(_, def)| def.models.iter().any(|m| m == model))
                    .map(|(name, _)| name.clone())
            })
            .unwrap_or_default()
    }

    /// List every model across every provider, with an `active` flag for
    /// the one the default agent uses.
    pub fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let config = self.storage().load_config()?;
        let active_model = self.active_model();
        let mut models = Vec::new();
        for (provider_name, def) in &config.provider {
            let kind: i32 = ProviderKind::from(&def.kind).into();
            for model_name in &def.models {
                models.push(ModelInfo {
                    name: model_name.clone(),
                    provider: provider_name.clone(),
                    active: *model_name == active_model,
                    kind,
                });
            }
        }
        Ok(models)
    }
}
