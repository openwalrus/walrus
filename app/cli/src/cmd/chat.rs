//! Interactive chat REPL command.

use crate::repl::ChatRepl;
use crate::runner::Runner;
use anyhow::Result;
use clap::Args;
use compact_str::CompactString;

/// Start an interactive chat REPL.
#[derive(Args, Debug)]
pub struct Chat;

impl Chat {
    /// Enter the interactive REPL with the given runner and agent.
    pub async fn run<R: Runner>(self, runner: R, agent: CompactString) -> Result<()> {
        let mut repl = ChatRepl::new(runner, agent)?;
        repl.run().await
    }
}
