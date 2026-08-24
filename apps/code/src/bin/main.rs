//! The crabtalk coding agent.

use anyhow::{Result, bail};

#[tokio::main]
async fn main() -> Result<()> {
    let Some(prompt) = parse()? else {
        return Ok(());
    };

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::WARN)
        .init();

    crabtalk_code::Crab::open(std::env::current_dir()?)
        .await?
        .turn(&prompt)
        .await
}

fn usage() -> String {
    let home = crabup::dirs::HOME_VAR;
    format!(
        "\
crab — the crabtalk coding agent

USAGE:
    crab -p <PROMPT>

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

/// The prompt to run, or `None` when the argument printed and we are done.
fn parse() -> Result<Option<String>> {
    let mut args = std::env::args().skip(1);
    let Some(arg) = args.next() else {
        print!("{}", usage());
        return Ok(None);
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
            Some(prompt) => Ok(Some(prompt)),
            None => bail!("-p takes a prompt\n\n{}", usage()),
        },
        other => bail!("unexpected argument: {other}\n\n{}", usage()),
    }
}
