//! Config management commands: show, set.

use crate::cmd::ConfigCommand;
use crate::config::resolve_config_path;
use anyhow::{Context, Result};

/// Dispatch config management subcommands.
pub fn run(config_flag: Option<&str>, action: &ConfigCommand) -> Result<()> {
    match action {
        ConfigCommand::Show => show(config_flag),
        ConfigCommand::Set { key, value } => set(config_flag, key, value),
    }
}

fn show(config_flag: Option<&str>) -> Result<()> {
    let path = resolve_config_path(config_flag);
    if !path.exists() {
        println!("No config file at {}", path.display());
        return Ok(());
    }
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    print!("{contents}");
    Ok(())
}

fn set(config_flag: Option<&str>, key: &str, value: &str) -> Result<()> {
    let path = resolve_config_path(config_flag);
    let contents = if path.exists() {
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };

    let mut doc: toml::Table = contents
        .parse()
        .with_context(|| format!("parsing {}", path.display()))?;

    // Support dotted keys: "llm.model" → doc["llm"]["model"].
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        [section, field] => {
            let table = doc
                .entry(*section)
                .or_insert_with(|| toml::Value::Table(toml::Table::new()))
                .as_table_mut()
                .ok_or_else(|| anyhow::anyhow!("'{section}' is not a table"))?;
            table.insert((*field).to_owned(), toml::Value::String(value.to_owned()));
        }
        [field] => {
            doc.insert((*field).to_owned(), toml::Value::String(value.to_owned()));
        }
        _ => anyhow::bail!("invalid key format: '{key}' (use 'section.field' or 'field')"),
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, doc.to_string())
        .with_context(|| format!("writing {}", path.display()))?;
    println!("Set {key} = {value} in {}", path.display());
    Ok(())
}
