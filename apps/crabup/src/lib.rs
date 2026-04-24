//! crabup — package manager for the Crabtalk ecosystem.

use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod cargo;
pub mod list;
pub mod registry;

#[derive(Parser, Debug)]
#[command(name = "crabup", about = "Crabtalk package and service manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Install a crabtalk binary from crates.io.
    Pull {
        /// Short name (daemon, tui, telegram, …) or crate name.
        name: String,
        /// Pin to a specific version.
        #[arg(long)]
        version: Option<String>,
    },
    /// Uninstall a crabtalk binary.
    Rm {
        /// Short name or crate name.
        name: String,
    },
    /// Bump every installed crabtalk-* crate to the latest version.
    Update,
    /// List installed crabtalk-* crates.
    List,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Command::Pull { name, version } => {
                cargo::install(registry::resolve(&name), version.as_deref())
            }
            Command::Rm { name } => cargo::uninstall(registry::resolve(&name)),
            Command::Update => {
                for krate in list::installed()? {
                    println!("==> {krate}");
                    cargo::install(&krate, None)?;
                }
                Ok(())
            }
            Command::List => {
                for krate in list::installed()? {
                    println!("{krate}");
                }
                Ok(())
            }
        }
    }
}
