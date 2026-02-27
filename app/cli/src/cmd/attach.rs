//! Attach to a remote gateway command.

use crate::repl::ChatRepl;
use crate::runner::gateway::GatewayRunner;
use anyhow::Result;
use clap::Args;
use compact_str::CompactString;
use std::path::PathBuf;

/// Attach to a running walrus-gateway via Unix domain socket.
#[derive(Args, Debug)]
pub struct Attach {
    /// Gateway socket path.
    #[arg(long)]
    pub socket: Option<PathBuf>,
}

impl Attach {
    /// Connect to the gateway and enter the interactive REPL.
    pub async fn run(self, agent: CompactString) -> Result<()> {
        let socket_path = self
            .socket
            .unwrap_or_else(|| gateway::config::global_config_dir().join("walrus.sock"));
        let runner = GatewayRunner::connect(&socket_path).await?;
        let mut repl = ChatRepl::new(runner, agent)?;
        repl.run().await
    }
}
