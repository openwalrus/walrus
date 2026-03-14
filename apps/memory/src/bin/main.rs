use clap::Parser;
use walrus_memory::cmd::{App, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let app = App::parse();
    match app.command {
        Command::Serve { socket } => walrus_memory::cmd::serve::run(&socket).await,
    }
}
