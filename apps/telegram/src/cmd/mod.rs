//! Telegram gateway CLI — clap command definitions.

use clap::Parser;

pub mod serve;

/// Crabtalk Telegram gateway service.
#[derive(Parser)]
#[command(name = "crabtalk-telegram")]
pub struct App {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Run the Telegram gateway, connecting to a crabtalk daemon.
    Serve {
        /// Daemon UDS socket path to connect to.
        #[arg(long)]
        daemon: String,
        /// JSON-encoded gateway config.
        #[arg(long, default_value = "{}")]
        config: String,
    },
}
