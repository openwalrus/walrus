//! CLI command definitions for wmemory.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod serve;

#[derive(Parser)]
#[command(name = "wmemory", about = "Walrus memory service")]
pub struct App {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run as a WHS hook service over UDS.
    Serve {
        /// UDS socket path to listen on.
        #[arg(long)]
        socket: PathBuf,
    },
}
