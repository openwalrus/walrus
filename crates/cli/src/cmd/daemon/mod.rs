//! `walrus daemon` — daemon lifecycle management.

use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::Path;

mod service;
mod start;

/// Manage the walrus daemon.
#[derive(Args, Debug)]
pub struct Daemon {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

#[derive(Subcommand, Debug)]
pub enum DaemonCommand {
    /// Start the daemon in the foreground.
    Start {
        /// Increase log verbosity (-v = info, -vv = debug, -vvv = trace).
        #[arg(short, long, action = clap::ArgAction::Count)]
        verbose: u8,
    },
    /// Trigger a hot-reload of the running daemon.
    Reload,
    /// Restart the daemon (requires system service).
    Restart,
    /// Install walrus as a system service (launchd/systemd).
    Install,
    /// Uninstall the walrus system service.
    Uninstall,
}

impl Daemon {
    pub async fn run(self, socket_path: &Path) -> Result<()> {
        match self.command {
            DaemonCommand::Start { .. } => start::start().await,
            DaemonCommand::Reload => reload(socket_path).await,
            DaemonCommand::Restart => service::restart(),
            DaemonCommand::Install => service::install(),
            DaemonCommand::Uninstall => service::uninstall(),
        }
    }
}

async fn reload(socket_path: &Path) -> Result<()> {
    let mut runner = crate::repl::runner::Runner::connect(socket_path).await?;
    runner.reload().await?;
    println!("daemon reloaded");
    Ok(())
}
