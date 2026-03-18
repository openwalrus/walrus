//! Interactive chat REPL with streaming output and persistent history.

use crate::repl::{
    command::{ReplHelper, handle_slash},
    runner::{OutputChunk, Runner},
};
use anyhow::Result;
use futures_core::Stream;
use futures_util::StreamExt;
use rustyline::{Editor, error::ReadlineError, history::DefaultHistory};
use std::{io::Write, path::PathBuf, pin::pin};

pub mod command;
pub mod runner;

/// Interactive chat REPL.
pub struct ChatRepl {
    runner: Runner,
    agent: String,
    editor: Editor<ReplHelper, DefaultHistory>,
    history_path: Option<PathBuf>,
}

impl ChatRepl {
    /// Create a new REPL with the given runner and agent name.
    pub fn new(runner: Runner, agent: String) -> Result<Self> {
        let mut editor = Editor::new()?;
        editor.set_helper(Some(ReplHelper));
        let history_path = history_file_path();
        if let Some(ref path) = history_path {
            let _ = editor.load_history(path);
        }
        Ok(Self {
            runner,
            agent,
            editor,
            history_path,
        })
    }

    /// Run the interactive REPL loop.
    pub async fn run(&mut self) -> Result<()> {
        println!("Walrus chat (Ctrl+D to exit, Ctrl+C to cancel)");
        println!("---");

        loop {
            match self.editor.readline("> ") {
                Ok(line) => {
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    let _ = self.editor.add_history_entry(&line);
                    if handle_slash(&mut self.agent, &line).await? {
                        continue;
                    }
                    let stream = self.runner.stream(&self.agent, &line);
                    stream_to_terminal(stream).await?;
                }
                Err(ReadlineError::Interrupted) => continue,
                Err(ReadlineError::Eof) => break,
                Err(e) => return Err(e.into()),
            }
        }

        self.save_history();
        Ok(())
    }

    /// Save readline history to disk.
    fn save_history(&mut self) {
        if let Some(ref path) = self.history_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = self.editor.save_history(path);
        }
    }
}

/// Resolve the history file path at `~/.openwalrus/history`.
fn history_file_path() -> Option<PathBuf> {
    Some(wcore::paths::CONFIG_DIR.join("history"))
}

/// ANSI escape: dim (gray) text.
const DIM: &str = "\x1b[2m";
/// ANSI escape: reset all attributes.
const RESET: &str = "\x1b[0m";

/// Consume a stream of output chunks and print them to stdout in real time.
///
/// Thinking chunks are rendered in dim/gray text.
/// Handles Ctrl+C cancellation via `tokio::signal::ctrl_c()`.
async fn stream_to_terminal(stream: impl Stream<Item = Result<OutputChunk>>) -> Result<()> {
    let mut stream = pin!(stream);
    let mut in_thinking = false;

    loop {
        tokio::select! {
            chunk = stream.next() => {
                match chunk {
                    Some(Ok(OutputChunk::Text(text))) => {
                        if in_thinking {
                            print!("{RESET}");
                            in_thinking = false;
                        }
                        print!("{text}");
                        std::io::stdout().flush().ok();
                    }
                    Some(Ok(OutputChunk::Thinking(text))) => {
                        if !in_thinking {
                            print!("{DIM}");
                            in_thinking = true;
                        }
                        print!("{text}");
                        std::io::stdout().flush().ok();
                    }
                    Some(Ok(OutputChunk::Status(text))) => {
                        if in_thinking {
                            print!("{RESET}");
                            in_thinking = false;
                        }
                        print!("{text}");
                        std::io::stdout().flush().ok();
                    }
                    Some(Err(e)) => {
                        if in_thinking {
                            print!("{RESET}");
                        }
                        eprintln!("\nError: {e}");
                        break;
                    }
                    None => break,
                }
            }
            _ = tokio::signal::ctrl_c() => {
                if in_thinking {
                    print!("{RESET}");
                }
                println!();
                break;
            }
        }
    }

    if in_thinking {
        print!("{RESET}");
    }
    println!();
    Ok(())
}
