//! Attach to a remote gateway command.

use crate::repl::ChatRepl;
use crate::runner::gateway::GatewayRunner;
use anyhow::Result;
use clap::Args;
use compact_str::CompactString;

/// Attach to a running walrus-gateway via WebSocket.
#[derive(Args, Debug)]
pub struct Attach {
    /// Gateway WebSocket URL.
    #[arg(long, default_value = "ws://127.0.0.1:6688/ws")]
    pub url: String,
    /// Authentication token.
    #[arg(long)]
    pub auth_token: Option<String>,
}

impl Attach {
    /// Connect to the gateway and enter the interactive REPL.
    pub async fn run(self, agent: CompactString) -> Result<()> {
        let runner = GatewayRunner::connect(&self.url, self.auth_token.as_deref()).await?;
        let mut repl = ChatRepl::new(runner, agent)?;
        repl.run().await
    }
}
