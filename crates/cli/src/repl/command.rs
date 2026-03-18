//! Slash command parsing, dispatch, and tab-completion for the REPL.

use anyhow::Result;
use rustyline::{
    Context,
    completion::{Completer, Pair},
};

pub const SLASH_COMMANDS: &[&str] = &["/help", "/switch"];

/// Rustyline helper providing tab-completion for slash commands.
#[derive(rustyline::Helper, rustyline::Hinter, rustyline::Highlighter, rustyline::Validator)]
pub struct ReplHelper;

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let prefix = &line[..pos];
        if !prefix.starts_with('/') {
            return Ok((0, vec![]));
        }
        let candidates = SLASH_COMMANDS
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|cmd| Pair {
                display: cmd.to_string(),
                replacement: cmd.to_string(),
            })
            .collect();
        Ok((0, candidates))
    }
}

/// Dispatch a slash command. Returns `true` if the line was handled.
pub async fn handle_slash(agent: &mut String, line: &str) -> Result<bool> {
    if !line.starts_with('/') {
        return Ok(false);
    }
    let rest = &line[1..];
    let (cmd, arg) = match rest.find(' ') {
        Some(pos) => (&rest[..pos], Some(rest[pos + 1..].trim())),
        None => (rest, None),
    };
    match cmd {
        "help" => {
            println!("Available commands:");
            println!("  /help          — show this help");
            println!("  /switch <name> — switch active agent");
        }
        "switch" => match arg {
            Some(name) if !name.is_empty() => {
                *agent = name.to_owned();
                println!("Switched to agent '{name}'.");
            }
            _ => println!("Usage: /switch <agent-name>"),
        },
        _ => println!("Unknown command '{cmd}'. Type /help for available commands."),
    }
    Ok(true)
}
