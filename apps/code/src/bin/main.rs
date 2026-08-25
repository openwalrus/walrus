//! The crabtalk coding agent.

use anyhow::{Result, bail};

#[tokio::main]
async fn main() -> Result<()> {
    let Some(run) = parse()? else {
        return Ok(());
    };

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::WARN)
        .init();

    let crab = crabtalk_code::Crab::open(std::env::current_dir()?).await?;
    match run {
        Run::Print(prompt) => crab.turn(&prompt).await,
        Run::Chat => crabtalk_code::tui::run(crab).await,
    }
}

/// What the arguments asked for.
enum Run {
    /// One turn, to stdout.
    Print(String),
    /// The terminal session.
    Chat,
}

fn usage() -> String {
    let home = crabup::dirs::HOME_VAR;
    format!(
        "\
crab — the crabtalk coding agent

USAGE:
    crab            Start a session in this directory
    crab -p <TEXT>  Run one turn and write the answer to stdout

OPTIONS:
    -p, --print    Run one turn and write the answer to stdout
    -h, --help     Print this message
    -V, --version  Print the version

ENVIRONMENT:
    {home}  Install root (default: ~/.crabtalk)

Tools are bound to the directory crab was started in.
"
    )
}

/// What to run, or `None` when the argument printed and we are done.
fn parse() -> Result<Option<Run>> {
    let mut args = std::env::args().skip(1);
    let Some(arg) = args.next() else {
        return Ok(Some(Run::Chat));
    };
    match arg.as_str() {
        "-h" | "--help" => {
            print!("{}", usage());
            Ok(None)
        }
        "-V" | "--version" => {
            println!("crab {}", env!("CARGO_PKG_VERSION"));
            Ok(None)
        }
        "-p" | "--print" => match args.next() {
            Some(prompt) => Ok(Some(Run::Print(prompt))),
            None => bail!("-p takes a prompt\n\n{}", usage()),
        },
        other => bail!("unexpected argument: {other}\n\n{}", usage()),
    }
}
