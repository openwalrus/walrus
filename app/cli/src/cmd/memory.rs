//! Memory management commands: list, get.

use crate::runner::gateway::GatewayRunner;
use anyhow::Result;
use clap::Subcommand;

/// Memory management subcommands.
#[derive(Subcommand, Debug)]
pub enum MemoryCommand {
    /// List all memory entries.
    List,
    /// Get a specific memory entry.
    Get {
        /// Memory key.
        key: String,
    },
}

impl MemoryCommand {
    /// Dispatch memory management subcommands.
    pub async fn run(&self, runner: &mut GatewayRunner) -> Result<()> {
        match self {
            Self::List => list(runner).await,
            Self::Get { key } => get(runner, key).await,
        }
    }
}

async fn list(runner: &mut GatewayRunner) -> Result<()> {
    let entries = runner.list_memory().await?;
    if entries.is_empty() {
        println!("No memory entries.");
        return Ok(());
    }
    for (key, value) in &entries {
        let preview = if value.len() > 80 {
            let end = value
                .char_indices()
                .nth(77)
                .map(|(i, _)| i)
                .unwrap_or(value.len());
            format!("{}...", &value[..end])
        } else {
            value.clone()
        };
        println!("  {key}: {preview}");
    }
    Ok(())
}

async fn get(runner: &mut GatewayRunner, key: &str) -> Result<()> {
    match runner.get_memory(key).await? {
        Some(value) => println!("{value}"),
        None => println!("No entry for key '{key}'."),
    }
    Ok(())
}
