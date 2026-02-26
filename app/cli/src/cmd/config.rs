//! Config management commands: show, set.

use crate::cmd::ConfigCommand;
use crate::prefs::CliPrefs;
use anyhow::Result;

/// Dispatch config management subcommands.
pub fn run(action: &ConfigCommand) -> Result<()> {
    match action {
        ConfigCommand::Show => show(),
        ConfigCommand::Set { key, value } => set(key, value),
    }
}

fn show() -> Result<()> {
    let prefs = CliPrefs::load()?;
    let toml = toml::to_string_pretty(&prefs)?;
    println!("{toml}");
    Ok(())
}

fn set(key: &str, value: &str) -> Result<()> {
    let mut prefs = CliPrefs::load()?;
    match key {
        "default_gateway" => prefs.default_gateway = Some(value.to_owned()),
        "default_agent" => prefs.default_agent = Some(value.to_owned()),
        "model" => prefs.model = Some(value.to_owned()),
        _ => anyhow::bail!(
            "unknown config key: '{key}'. Valid keys: default_gateway, default_agent, model"
        ),
    }
    prefs.save()?;
    println!("Set {key} = {value}");
    Ok(())
}
