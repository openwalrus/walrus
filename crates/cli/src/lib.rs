//! Cydonia CLI

use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt};
pub use {chat::ChatCmd, config::Config};

mod chat;
mod config;

/// Cydonia CLI
#[derive(Debug, Parser)]
#[command(name = "cydonia", version, about)]
pub struct App {
    /// Enable streaming mode
    #[arg(short, long, global = true)]
    pub stream: bool,

    /// Verbosity level (use -v, -vv, -vvv, etc.)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Subcommand to run
    #[command(subcommand)]
    pub command: Command,
}

/// Available commands
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Chat with an LLM
    Chat(chat::ChatCmd),

    /// Generate the configuration file
    Generate,
}

impl App {
    /// Initialize tracing subscriber based on verbosity
    pub fn init_tracing(&self) {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            let directive = match self.verbose {
                0 => "info",
                1 => "cydonia=debug",
                2 => "cydonia=trace",
                3 => "debug",
                _ => "trace",
            };
            EnvFilter::new(directive)
        });

        fmt()
            .without_time()
            .with_env_filter(filter)
            .with_target(self.verbose != 0)
            .init();
    }
}
