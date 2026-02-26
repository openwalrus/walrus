//! Interactive chat REPL with streaming output and persistent history.

use crate::runner::Runner;
use anyhow::Result;
use compact_str::CompactString;
use futures_core::Stream;
use futures_util::StreamExt;
use rustyline::error::ReadlineError;
use std::{io::Write, path::PathBuf, pin::pin};

/// Interactive chat REPL, generic over the execution backend.
pub struct ChatRepl<R: Runner> {
    runner: R,
    agent: CompactString,
    editor: rustyline::DefaultEditor,
    history_path: Option<PathBuf>,
}

impl<R: Runner> ChatRepl<R> {
    /// Create a new REPL with the given runner and agent name.
    pub fn new(runner: R, agent: CompactString) -> Result<Self> {
        let mut editor = rustyline::DefaultEditor::new()?;
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

/// Resolve the history file path at `~/.config/walrus/history`.
fn history_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("walrus").join("history"))
}

/// Consume a stream of content chunks and print them to stdout in real time.
///
/// Handles Ctrl+C cancellation via `tokio::signal::ctrl_c()`.
async fn stream_to_terminal(stream: impl Stream<Item = Result<String>>) -> Result<()> {
    let mut stream = pin!(stream);

    loop {
        tokio::select! {
            chunk = stream.next() => {
                match chunk {
                    Some(Ok(text)) => {
                        print!("{text}");
                        std::io::stdout().flush().ok();
                    }
                    Some(Err(e)) => {
                        eprintln!("\nError: {e}");
                        break;
                    }
                    None => break,
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!();
                break;
            }
        }
    }

    println!();
    Ok(())
}
