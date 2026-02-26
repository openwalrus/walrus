//! Walrus CLI binary entry point.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;
use walrus_cli::{
    Cli, Command, cmd,
    repl::ChatRepl,
    runner::{Runner, direct::DirectRunner, gateway::GatewayRunner},
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Attach mode: connect to a running gateway via WebSocket.
    if let Command::Attach { ref url, ref auth_token } = cli.command {
        let runner = GatewayRunner::connect(url, auth_token.as_deref()).await?;
        return run_with_runner(runner, &cli).await;
    }

    // Direct mode: embed the full gateway stack locally.
    let runner = DirectRunner::new(&cli).await?;

    // Management commands.
    match &cli.command {
        Command::Agent { action } => return cmd::agent::run(&runner, action),
        Command::Memory { action } => return cmd::memory::run(&runner, action),
        Command::Config { action } => return cmd::config::run(action),
        _ => {}
    }

    run_with_runner(runner, &cli).await
}

/// Dispatch chat/send subcommands using a concrete runner.
async fn run_with_runner<R: Runner>(mut runner: R, cli: &Cli) -> Result<()> {
    match cli.command {
        Command::Chat | Command::Attach { .. } => {
            let agent = cli
                .agent
                .as_deref()
                .map(|s| s.into())
                .unwrap_or_else(|| "assistant".into());
            let mut repl = ChatRepl::new(runner, agent)?;
            repl.run().await?;
        }
        Command::Send { ref content } => {
            let agent = cli.agent.as_deref().unwrap_or("assistant");
            let response = runner.send(agent, content).await?;
            println!("{response}");
        }
        // Management commands handled above.
        Command::Agent { .. } | Command::Memory { .. } | Command::Config { .. } => {
            unreachable!()
        }
    }

    Ok(())
}
