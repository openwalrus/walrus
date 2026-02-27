//! CLI argument parsing and command dispatch.

use crate::runner::gateway::GatewayRunner;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use compact_str::CompactString;
use std::path::PathBuf;

pub mod agent;
pub mod chat;
pub mod config;
pub mod memory;
pub mod send;

/// Walrus CLI client — connects to walrusd via Unix domain socket.
#[derive(Parser, Debug)]
#[command(name = "walrus", about = "Walrus CLI client")]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Command,

    /// Agent name override.
    #[arg(long, global = true)]
    pub agent: Option<CompactString>,

    /// Path to the walrusd socket.
    #[arg(long, global = true)]
    pub socket: Option<PathBuf>,
}

impl Cli {
    /// Resolve the agent name from CLI flags or fall back to "assistant".
    pub fn resolve_agent(&self) -> CompactString {
        self.agent.clone().unwrap_or_else(|| "assistant".into())
    }

    /// Resolve the socket path from CLI flag or default.
    fn resolve_socket(&self) -> PathBuf {
        self.socket.clone().unwrap_or_else(|| {
            dirs::config_dir()
                .expect("no platform config directory")
                .join("walrus")
                .join("walrus.sock")
        })
    }

    /// Parse and dispatch the CLI command.
    pub async fn run(self) -> Result<()> {
        let agent = self.resolve_agent();
        let socket_path = self.resolve_socket();
        match self.command {
            Command::Config(cmd) => cmd.run(),
            Command::Chat(cmd) => {
                let runner = connect(&socket_path).await?;
                cmd.run(runner, agent).await
            }
            Command::Send(cmd) => {
                let mut runner = connect(&socket_path).await?;
                cmd.run(&mut runner, &agent).await
            }
            Command::Agent(cmd) => {
                let mut runner = connect(&socket_path).await?;
                cmd.run(&mut runner).await
            }
            Command::Memory(cmd) => {
                let mut runner = connect(&socket_path).await?;
                cmd.run(&mut runner).await
            }
        }
    }
}

/// Top-level subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start an interactive chat REPL.
    Chat(chat::Chat),
    /// Send a one-shot message to an agent.
    Send(send::Send),
    /// Manage agents.
    #[command(subcommand)]
    Agent(agent::AgentCommand),
    /// Manage memory entries.
    #[command(subcommand)]
    Memory(memory::MemoryCommand),
    /// Manage configuration.
    #[command(subcommand)]
    Config(config::ConfigCommand),
}

/// Connect to walrusd, returning a helpful error if not running.
async fn connect(socket_path: &std::path::Path) -> Result<GatewayRunner> {
    GatewayRunner::connect(socket_path).await.with_context(|| {
        format!(
            "failed to connect to walrusd at {}. Is walrusd running?",
            socket_path.display()
        )
    })
}
