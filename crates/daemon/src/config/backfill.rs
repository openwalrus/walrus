//! One-shot migration that backfills `[agents.<name>].id` stanzas in
//! `local/CrabTalk.toml` for installs that predate [`AgentId`].
//!
//! Runs at daemon startup before manifests are resolved. Only the local
//! manifest is touched — plugin manifests come from git repos and
//! mutating them would drift from their sources. Plugin-sourced agents
//! without an `id` field get the `AgentId::nil()` sentinel until a
//! later phase decides how to persist their identity.

use anyhow::{Context, Result};
use std::path::Path;
use toml_edit::{DocumentMut, Item, value};
use ulid::Ulid;
use wcore::paths::LOCAL_DIR;

/// Ensure every `[agents.<name>]` entry in `local/CrabTalk.toml` has an
/// `id` field. Missing entries get a fresh ULID; the file is rewritten
/// in place (preserving comments and formatting via `toml_edit`) only
/// if at least one backfill happened.
pub fn backfill_local_agent_ids(config_dir: &Path) -> Result<()> {
    let manifest_path = config_dir.join(LOCAL_DIR).join("CrabTalk.toml");
    if !manifest_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("parse {}", manifest_path.display()))?;

    let mut mutated = 0usize;
    if let Some(agents_item) = doc.get_mut("agents")
        && let Some(agents) = agents_item.as_table_like_mut()
    {
        for (name, item) in agents.iter_mut() {
            if let Item::Table(table) = item {
                if !table.contains_key("id") {
                    let id = Ulid::new().to_string();
                    table.insert("id", value(id.clone()));
                    tracing::info!("backfilled agent id for '{name}': {id}");
                    mutated += 1;
                }
            } else if let Item::Value(toml_edit::Value::InlineTable(inline)) = item
                && !inline.contains_key("id")
            {
                let id = Ulid::new().to_string();
                inline.insert("id", toml_edit::Value::from(id.clone()));
                tracing::info!("backfilled agent id for '{name}': {id}");
                mutated += 1;
            }
        }
    }

    if mutated > 0 {
        std::fs::write(&manifest_path, doc.to_string())
            .with_context(|| format!("write {}", manifest_path.display()))?;
        tracing::info!(
            "backfilled {mutated} agent id(s) in {}",
            manifest_path.display()
        );
    }
    Ok(())
}
