//! CLI argument parsing and management subcommand handlers.

use clap::{Parser, Subcommand};
use compact_str::CompactString;

pub mod agent;
pub mod config;
pub mod memory;

/// Walrus AI agent platform.
#[derive(Parser, Debug)]
#[command(name = "walrus", about = "Walrus AI agent platform")]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Command,

    /// Model name override.
    #[arg(long, global = true)]
    pub model: Option<CompactString>,

    /// Agent name override.
    #[arg(long, global = true)]
    pub agent: Option<CompactString>,
}

/// Top-level subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start an interactive chat REPL.
    Chat,
    /// Send a one-shot message to an agent.
    Send {
        /// Message content.
        content: String,
    },
    /// Manage agents.
    Agent {
        /// Agent subcommand.
        #[command(subcommand)]
        action: AgentCommand,
    },
    /// Manage memory entries.
    Memory {
        /// Memory subcommand.
        #[command(subcommand)]
        action: MemoryCommand,
    },
    /// Manage CLI configuration.
    Config {
        /// Config subcommand.
        #[command(subcommand)]
        action: ConfigCommand,
    },
    /// Attach to a running walrus-gateway via WebSocket.
    Attach {
        /// Gateway WebSocket URL.
        #[arg(long, default_value = "ws://127.0.0.1:6688/ws")]
        url: String,
        /// Authentication token.
        #[arg(long)]
        auth_token: Option<String>,
    },
}

/// Agent management subcommands.
#[derive(Subcommand, Debug)]
pub enum AgentCommand {
    /// List registered agents.
    List,
    /// Show agent details.
    Info {
        /// Agent name.
        name: String,
    },
}

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

/// Config management subcommands.
#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Show current configuration.
    Show,
    /// Set a configuration value.
    Set {
        /// Configuration key.
        key: String,
        /// Configuration value.
        value: String,
    },
}
