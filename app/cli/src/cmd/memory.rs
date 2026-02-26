//! Memory management commands: list, get.

use crate::runner::direct::DirectRunner;
use anyhow::Result;
use clap::Subcommand;
use runtime::Memory;

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
    pub fn run(&self, runner: &DirectRunner) -> Result<()> {
        match self {
            Self::List => list(runner),
            Self::Get { key } => get(runner, key),
        }
    }
}

fn list(runner: &DirectRunner) -> Result<()> {
    let entries = runner.runtime.memory().entries();
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

fn get(runner: &DirectRunner, key: &str) -> Result<()> {
    match runner.runtime.memory().get(key) {
        Some(value) => println!("{value}"),
        None => println!("No entry for key '{key}'."),
    }
    Ok(())
}
