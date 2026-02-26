//! One-shot message command.

use crate::runner::Runner;
use anyhow::Result;
use clap::Args;

/// Send a one-shot message to an agent.
#[derive(Args, Debug)]
pub struct Send {
    /// Message content.
    pub content: String,
}

impl Send {
    /// Send a message and print the response.
    pub async fn run(self, runner: &mut impl Runner, agent: &str) -> Result<()> {
        let response = runner.send(agent, &self.content).await?;
        println!("{response}");
        Ok(())
    }
}
