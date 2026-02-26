//! CLI argument parsing and command dispatch.

use crate::runner::direct::DirectRunner;
use anyhow::Result;
use clap::{Parser, Subcommand};
use compact_str::CompactString;

pub mod agent;
pub mod attach;
pub mod chat;
pub mod config;
pub mod memory;
pub mod send;
pub mod serve;

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

impl Cli {
    /// Resolve the agent name from CLI flags or fall back to "assistant".
    pub fn resolve_agent(&self) -> CompactString {
        self.agent.clone().unwrap_or_else(|| "assistant".into())
    }

    /// Parse and dispatch the CLI command.
    pub async fn run(self) -> Result<()> {
        let agent = self.resolve_agent();
        match self.command {
            Command::Serve(cmd) => cmd.run().await,
            Command::Attach(cmd) => cmd.run(agent).await,
            Command::Config(cmd) => cmd.run(),
            Command::Chat(cmd) => {
                let runner = DirectRunner::new().await?;
                cmd.run(runner, agent).await
            }
            Command::Send(cmd) => {
                let mut runner = DirectRunner::new().await?;
                cmd.run(&mut runner, &agent).await
            }
            Command::Agent(cmd) => {
                let runner = DirectRunner::new().await?;
                cmd.run(&runner)
            }
            Command::Memory(cmd) => {
                let runner = DirectRunner::new().await?;
                cmd.run(&runner)
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
    /// Manage CLI configuration.
    #[command(subcommand)]
    Config(config::ConfigCommand),
    /// Start the gateway server.
    Serve(serve::Serve),
    /// Attach to a running walrus-gateway via WebSocket.
    Attach(attach::Attach),
}
