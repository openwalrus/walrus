//! Model configuration.
//!
//! `ProviderDef` and `ApiStandard` are defined in wcore and re-exported here.

use anyhow::{Result, bail};
use std::collections::{BTreeMap, HashSet};

pub use wcore::config::provider::{ApiStandard, ProviderDef};

/// Validate provider definitions and reject duplicate model names.
pub fn validate_providers(providers: &BTreeMap<String, ProviderDef>) -> Result<()> {
    let mut seen = HashSet::new();
    for (name, def) in providers {
        def.validate(name).map_err(|e| anyhow::anyhow!(e))?;
        for model in &def.models {
            if !seen.insert(model.clone()) {
                bail!("duplicate model name '{model}' across providers");
            }
        }
    }
    Ok(())
}
