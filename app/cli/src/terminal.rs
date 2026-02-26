//! Terminal output utilities for streaming responses.

use anyhow::Result;
use futures_core::Stream;
use futures_util::StreamExt;
use std::io::Write;
use std::pin::pin;

/// Consume a stream of content chunks and print them to stdout in real time.
///
/// Handles Ctrl+C cancellation via `tokio::signal::ctrl_c()`.
pub async fn stream_to_terminal(stream: impl Stream<Item = Result<String>>) -> Result<()> {
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
