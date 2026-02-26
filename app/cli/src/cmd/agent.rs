//! Agent management commands: list, info.

use crate::cli::AgentCommand;
use crate::direct::DirectRunner;
use anyhow::Result;

/// Dispatch agent management subcommands.
pub fn run(runner: &DirectRunner, action: &AgentCommand) -> Result<()> {
    match action {
        AgentCommand::List => list(runner),
        AgentCommand::Info { name } => info(runner, name),
    }
}

fn list(runner: &DirectRunner) -> Result<()> {
    let agents: Vec<_> = runner.runtime.agents().collect();
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

fn info(runner: &DirectRunner, name: &str) -> Result<()> {
    let agent = runner
        .runtime
        .agent(name)
        .ok_or_else(|| anyhow::anyhow!("agent '{}' not found", name))?;

    println!("Name:        {}", agent.name);
    println!("Description: {}", agent.description);
    let tools = if agent.tools.is_empty() {
        "(none)".to_owned()
    } else {
        agent.tools.join(", ")
    };
    let tags = if agent.skill_tags.is_empty() {
        "(none)".to_owned()
    } else {
        agent.skill_tags.join(", ")
    };
    println!("Tools:       {tools}");
    println!("Skill tags:  {tags}");
    if !agent.system_prompt.is_empty() {
        println!("\nSystem prompt:\n{}", agent.system_prompt);
    }
    Ok(())
}
