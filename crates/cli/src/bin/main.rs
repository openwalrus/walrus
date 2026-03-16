//! Walrus CLI binary entry point.

use anyhow::Result;
use clap::Parser;
use openwalrus::Cli;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = match cli.log_filter() {
        Some(f) => {
            // Set RUST_LOG so spawned child services inherit the same level.
            // SAFETY: called in main before spawning any threads.
            unsafe { std::env::set_var("RUST_LOG", f) };
            EnvFilter::new(f)
        }
        None => EnvFilter::from_default_env(),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    cli.run().await
}
