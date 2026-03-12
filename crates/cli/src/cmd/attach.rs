//! Attach to an agent via the interactive chat REPL.

use crate::repl::{ChatRepl, runner::Runner};
use anyhow::Result;
use clap::Args;
use compact_str::CompactString;

/// Attach to an agent and start an interactive chat REPL.
#[derive(Args, Debug)]
pub struct Attach {
    /// Connect via TCP instead of Unix domain socket.
    /// Reads the port from ~/.openwalrus/walrus.tcp.
    #[arg(long, default_missing_value = "true", num_args = 0)]
    pub tcp: bool,
}

impl Attach {
    /// Enter the interactive REPL with the given runner and agent.
    pub async fn run(self, runner: Runner, agent: CompactString) -> Result<()> {
        let mut repl = ChatRepl::new(runner, agent)?;
        repl.run().await
    }
}
