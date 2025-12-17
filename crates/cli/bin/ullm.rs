use anyhow::Result;
use clap::Parser;
use ucli::{App, Command, Config};

#[tokio::main]
async fn main() -> Result<()> {
    let app = App::parse();
    app.init_tracing();

    match app.command {
        Command::Chat(chat) => chat.run(app.stream).await?,
        Command::Generate => Config::default().save()?,
    }

    Ok(())
}
