//! The chat loop, inline in the terminal.
//!
//! The rule the whole loop follows: **the viewport holds what is
//! unfinished, scrollback holds what is final.** A settled item is
//! written once with `insert_before` and belongs to the terminal after
//! that — its scrolling, its selection, its copy. Nothing here redraws
//! it, and nothing here can.

use crate::{
    crab::Crab,
    tui::{
        input::{Action, History, Input},
        item::Item,
        stream::Transcript,
    },
};
use anyhow::Result;
use crossterm::{
    event::{Event as TermEvent, EventStream, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use proto::stream_event::Event as Chunk;
use ratatui::{
    Frame, Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use std::{io::Stdout, time::Duration};
use tokio::sync::mpsc;

mod input;
mod item;
mod markdown;
mod stream;

/// Rows the viewport keeps for what has not settled: the input box, the
/// status line, and the block currently streaming. ratatui cannot resize
/// an inline viewport after it is built, so this is a ceiling rather than
/// a preference — a taller input scrolls inside it.
const VIEWPORT: u16 = 8;

/// How often the spinner advances while a tool runs.
const TICK: Duration = Duration::from_millis(80);

type Screen = Terminal<CrosstermBackend<Stdout>>;

pub async fn run(crab: Crab) -> Result<()> {
    enable_raw_mode()?;
    // A panic past this point would otherwise leave the terminal raw and
    // the shell unusable, so put it back before the message is printed.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        hook(info);
    }));

    let terminal = Terminal::with_options(
        CrosstermBackend::new(std::io::stdout()),
        TerminalOptions {
            viewport: Viewport::Inline(VIEWPORT),
        },
    );
    let mut terminal = match terminal {
        Ok(terminal) => terminal,
        Err(e) => {
            let _ = disable_raw_mode();
            return Err(e.into());
        }
    };

    let mut app = App::new(crab, terminal.size()?.width as usize);
    let result = app.run(&mut terminal).await;

    disable_raw_mode()?;
    // Leave the viewport behind rather than on top of the prompt.
    terminal.clear()?;
    app.input.history.save(&history_path());
    result
}

fn history_path() -> std::path::PathBuf {
    crabup::dirs::CONFIG_DIR.join("code/history")
}

struct App {
    crab: Crab,
    transcript: Transcript,
    input: Input,
    /// The turn in flight, if there is one. Dropping it walks away.
    turn: Option<mpsc::UnboundedReceiver<Result<Chunk>>>,
    frame: u64,
}

impl App {
    fn new(crab: Crab, width: usize) -> Self {
        Self {
            transcript: Transcript::new(width),
            input: Input::new(History::load(&history_path())),
            turn: None,
            frame: 0,
            crab,
        }
    }

    async fn run(&mut self, terminal: &mut Screen) -> Result<()> {
        let mut keys = EventStream::new();
        let mut tick = tokio::time::interval(TICK);

        loop {
            self.commit(terminal)?;
            terminal.draw(|frame| self.view(frame))?;

            tokio::select! {
                Some(event) = keys.next() => {
                    match event? {
                        TermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                            if self.key(key) {
                                return Ok(());
                            }
                        }
                        TermEvent::Resize(width, _) => self.transcript.set_width(width as usize),
                        _ => {}
                    }
                }
                Some(event) = turn(&mut self.turn) => self.chunk(event?),
                _ = tick.tick() => self.frame += 1,
            }
        }
    }

    /// `true` to leave.
    fn key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        match self.input.key(key) {
            Action::Submit(prompt) if !prompt.trim().is_empty() => {
                self.transcript.push(Item::Prompt {
                    text: prompt.clone(),
                });
                self.transcript.push(Item::Blank);
                self.transcript.start();
                self.turn = Some(self.crab.spawn_turn(&prompt));
            }
            Action::Submit(_) => {}
            // A turn walks away; an idle Ctrl+C only clears the line.
            Action::Interrupt => match self.turn.take() {
                Some(_) => self.transcript.finish(),
                None => self.input.clear(),
            },
            Action::Eof => return true,
            Action::Noop => {}
        }
        false
    }

    fn chunk(&mut self, chunk: Chunk) {
        match chunk {
            Chunk::Chunk(text) => self.transcript.push_text(&text.content),
            Chunk::Thinking(text) => self.transcript.push_thinking(&text.content),
            Chunk::ToolStart(start) => {
                let calls: Vec<(String, String)> = start
                    .calls
                    .iter()
                    .map(|call| (call.name.clone(), call.arguments.clone()))
                    .collect();
                self.transcript.push_tool_start(&calls);
            }
            Chunk::ToolResult(result) => self.transcript.push_tool_result(&result.output),
            Chunk::ToolsComplete(_) => self.transcript.push_tool_done(),
            Chunk::End(end) => {
                if !end.error.is_empty() {
                    self.transcript.push_text(&format!("\n{}\n", end.error));
                }
                self.transcript.finish();
                self.turn = None;
            }
            _ => {}
        }
    }

    /// Hand everything final to the terminal, one item at a time.
    fn commit(&mut self, terminal: &mut Screen) -> Result<()> {
        let width = terminal.size()?.width as usize;
        let mut lines = Vec::new();
        for item in self.transcript.settled() {
            lines.extend(item.render(width, self.frame));
        }
        if lines.is_empty() {
            return Ok(());
        }
        terminal.insert_before(lines.len() as u16, |buf| {
            Paragraph::new(lines).render(buf.area, buf);
        })?;
        Ok(())
    }

    fn view(&self, frame: &mut Frame) {
        let area = frame.area();
        let input_height = self.input.height().min(area.height.saturating_sub(1));
        let [live, box_area, status] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(input_height),
            Constraint::Length(1),
        ])
        .areas(area);

        self.live(frame, live);
        self.input.render(frame, box_area, "crab");
        frame.render_widget(Paragraph::new(self.status()), status);
    }

    /// What is still in flight, anchored to the bottom so it sits against
    /// the input box and the blank room is up against the scrollback.
    fn live(&self, frame: &mut Frame, area: Rect) {
        let width = area.width as usize;
        let mut lines: Vec<Line> = self
            .transcript
            .live()
            .iter()
            .flat_map(|item| item.render(width, self.frame))
            .collect();
        if let Some(current) = self.transcript.current() {
            lines.push(current);
        }
        let height = area.height as usize;
        if lines.len() > height {
            lines.drain(..lines.len() - height);
        }
        let top = area.height.saturating_sub(lines.len() as u16);
        frame.render_widget(
            Paragraph::new(lines),
            Rect {
                y: area.y + top,
                height: area.height - top,
                ..area
            },
        );
    }

    fn status(&self) -> Line<'static> {
        let dim = Style::new().add_modifier(Modifier::DIM);
        let state = match (self.turn.is_some(), self.transcript.waiting) {
            (true, true) => "thinking",
            (true, false) => "working",
            _ => "ready",
        };
        Line::from(vec![
            Span::styled(format!("  {state}"), dim),
            Span::styled(
                format!("  ·  {}  ·  ctrl+d to exit", self.crab.root.display()),
                dim,
            ),
        ])
    }
}

/// The turn's next event, or never when there is no turn.
async fn turn(turn: &mut Option<mpsc::UnboundedReceiver<Result<Chunk>>>) -> Option<Result<Chunk>> {
    match turn {
        Some(events) => events.recv().await,
        None => std::future::pending().await,
    }
}
