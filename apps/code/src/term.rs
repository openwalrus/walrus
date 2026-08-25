//! The terminal, in raw mode with an inline viewport, and put back after.
//!
//! Two screens need this — naming an endpoint and talking to one — and
//! the sequence is unforgiving in both: raw mode has to come off however
//! the screen ends, including through a panic, or the shell that started
//! it is left unusable.

use anyhow::Result;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::{Terminal, TerminalOptions, Viewport, backend::CrosstermBackend};
use std::{
    io::Stdout,
    ops::{Deref, DerefMut},
    sync::Once,
};

/// Installed once. Wrapping the hook a second time would chain a second
/// copy of itself onto the first.
static HOOK: Once = Once::new();

pub struct Inline {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Inline {
    /// Take the terminal, keeping `rows` of it for the viewport.
    ///
    /// ratatui cannot resize an inline viewport once it is built, so
    /// `rows` is the tallest the screen will ever be rather than what it
    /// starts at.
    pub fn open(rows: u16) -> Result<Self> {
        enable_raw_mode()?;
        // The panic message is printed before unwinding reaches `drop`,
        // so it would land on a raw terminal without this.
        HOOK.call_once(|| {
            let hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let _ = disable_raw_mode();
                hook(info);
            }));
        });

        let terminal = Terminal::with_options(
            CrosstermBackend::new(std::io::stdout()),
            TerminalOptions {
                viewport: Viewport::Inline(rows),
            },
        );
        match terminal {
            Ok(terminal) => Ok(Self { terminal }),
            Err(e) => {
                let _ = disable_raw_mode();
                Err(e.into())
            }
        }
    }
}

impl Drop for Inline {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        // Leave the viewport behind rather than on top of the prompt.
        let _ = self.terminal.clear();
    }
}

impl Deref for Inline {
    type Target = Terminal<CrosstermBackend<Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Inline {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}
