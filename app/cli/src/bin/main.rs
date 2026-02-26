//! Walrus CLI binary entry point.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;
use walrus_cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Cli::parse().run().await
}
