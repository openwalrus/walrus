//! Walrus CLI binary entry point.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;
use walrus_cli::{
    Cli, Command, cmd, direct::DirectRunner, gateway::GatewayRunner, repl::ChatRepl, runner::Runner,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Gateway mode: connect to a remote gateway via WebSocket.
    if let Some(ref gateway_url) = cli.gateway {
        let auth_token = None; // TODO: read from config or --token flag
        let runner = GatewayRunner::connect(gateway_url, auth_token).await?;
        return run_with_runner(runner, &cli).await;
    }

    // Direct mode: embed the full gateway stack locally.
    let runner = DirectRunner::new(&cli).await?;

    // Management commands (direct-mode only).
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
        Command::Chat => {
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
        Command::Init => todo!("P5: workspace init"),
        Command::Attach => todo!("future: attach to gateway session"),
        Command::Hub => todo!("P6: hub commands"),
        // Management commands handled above in direct mode.
        Command::Agent { .. } | Command::Memory { .. } | Command::Config { .. } => {
            anyhow::bail!("management commands require direct mode (remove --gateway flag)")
        }
    }

    Ok(())
}
