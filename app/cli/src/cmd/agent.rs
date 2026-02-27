//! Agent management commands: list, info.

use crate::runner::gateway::GatewayRunner;
use anyhow::Result;
use clap::Subcommand;
use protocol::ServerMessage;

/// Agent management subcommands.
#[derive(Subcommand, Debug)]
pub enum AgentCommand {
    /// List registered agents.
    List,
    /// Show agent details.
    Info {
        /// Agent name.
        name: String,
    },
}

impl AgentCommand {
    /// Dispatch agent management subcommands.
    pub async fn run(&self, runner: &mut GatewayRunner) -> Result<()> {
        match self {
            Self::List => list(runner).await,
            Self::Info { name } => info(runner, name).await,
        }
    }
}

async fn list(runner: &mut GatewayRunner) -> Result<()> {
    let agents = runner.list_agents().await?;
    if agents.is_empty() {
        println!("No agents registered.");
        return Ok(());
    }
    for agent in agents {
        let desc = if agent.description.is_empty() {
            "(no description)"
        } else {
            agent.description.as_str()
        };
        println!("  {} — {}", agent.name, desc);
    }
    Ok(())
}

async fn info(runner: &mut GatewayRunner, name: &str) -> Result<()> {
    match runner.agent_info(name).await? {
        ServerMessage::AgentDetail {
            name,
            description,
            tools,
            skill_tags,
            system_prompt,
        } => {
            println!("Name:        {name}");
            println!("Description: {description}");
            let tools_str = if tools.is_empty() {
                "(none)".to_owned()
            } else {
                tools
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let tags_str = if skill_tags.is_empty() {
                "(none)".to_owned()
            } else {
                skill_tags
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            println!("Tools:       {tools_str}");
            println!("Skill tags:  {tags_str}");
            if !system_prompt.is_empty() {
                println!("\nSystem prompt:\n{system_prompt}");
            }
        }
        ServerMessage::Error { code, message } => {
            anyhow::bail!("error ({code}): {message}");
        }
        other => anyhow::bail!("unexpected response: {other:?}"),
    }
    Ok(())
}
