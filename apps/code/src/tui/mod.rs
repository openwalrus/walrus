//! The chat loop, inline in the terminal.
//!
//! The rule the whole loop follows: **the viewport holds what is
//! unfinished, scrollback holds what is final.** A settled item is
//! written once with `insert_before` and belongs to the terminal after
//! that — its scrolling, its selection, its copy. Nothing here redraws
//! it, and nothing here can.

use crate::{
    crab::Crab,
    term,
    tui::{
        command::{Command, HELP},
        input::{Action, History, Input},
        item::Item,
        stream::Transcript,
    },
};
use anyhow::Result;
use crossterm::event::{Event as TermEvent, EventStream, KeyEvent, KeyEventKind};
use futures_util::StreamExt;
use proto::stream_event::Event as Chunk;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use std::{io::Stdout, time::Duration};
use tokio::sync::mpsc;

mod command;
mod input;
mod item;
mod markdown;
mod replay;
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
    let mut terminal = term::Inline::open(VIEWPORT)?;
    let mut app = App::new(crab, terminal.size()?.width as usize);
    let result = app.run(&mut terminal).await;
    app.input.history.save(&history_path());
    result
}

fn history_path() -> std::path::PathBuf {
    crabup::dirs::CONFIG_DIR.join("code/history")
}

/// What the loop woke for. Read out of the select rather than handled
/// inside it, so a handler can borrow the whole app.
enum Wake {
    Term(TermEvent),
    Turn(Result<Chunk>),
    Tick,
}

struct App {
    crab: Crab,
    transcript: Transcript,
    input: Input,
    /// The turn in flight, if there is one. Dropping it walks away.
    turn: Option<mpsc::UnboundedReceiver<Result<Chunk>>>,
    /// Prompt tokens the last call reported. Shown rather than acted on:
    /// nothing here knows the model's window, so what to do about the
    /// number is the developer's call.
    tokens: Option<u32>,
    /// Set when a turn ends over the window, acted on by the loop —
    /// summarizing is a round trip and the event handler is not async.
    compact_due: bool,
    frame: u64,
}

impl App {
    fn new(crab: Crab, width: usize) -> Self {
        Self {
            transcript: Transcript::new(width),
            input: Input::new(History::load(&history_path())),
            turn: None,
            tokens: None,
            compact_due: false,
            frame: 0,
            crab,
        }
    }

    async fn run(&mut self, terminal: &mut Screen) -> Result<()> {
        self.replay(terminal).await?;
        let mut keys = EventStream::new();
        let mut tick = tokio::time::interval(TICK);

        loop {
            self.commit(terminal)?;
            terminal.draw(|frame| self.view(frame))?;

            let wake = tokio::select! {
                Some(event) = keys.next() => Wake::Term(event?),
                Some(event) = turn(&mut self.turn) => Wake::Turn(event),
                _ = tick.tick() => Wake::Tick,
            };

            match wake {
                Wake::Term(TermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
                    if self.key(key).await? {
                        return Ok(());
                    }
                }
                Wake::Term(TermEvent::Resize(width, _)) => {
                    self.transcript.set_width(width as usize)
                }
                Wake::Term(_) => {}
                Wake::Turn(Ok(chunk)) => self.chunk(chunk),
                Wake::Turn(Err(e)) => self.fail(e.to_string()),
                Wake::Tick => self.frame += 1,
            }

            if std::mem::take(&mut self.compact_due) {
                self.compact(true).await;
            }
        }
    }

    /// Put the stored session back on screen, so resuming shows the
    /// conversation the model is already holding.
    async fn replay(&mut self, terminal: &mut Screen) -> Result<()> {
        let history = self.crab.history().await?;
        if history.is_empty() {
            return Ok(());
        }
        let width = terminal.size()?.width as usize;
        let mut lines: Vec<Line> = replay::items(&history)
            .iter()
            .flat_map(|item| item.render(width, 0))
            .collect();

        if lines.len() > replay::MAX_ROWS {
            let dropped = lines.len() - replay::MAX_ROWS;
            lines.drain(..dropped);
            lines.insert(
                0,
                Line::from(Span::styled(
                    format!("  … {dropped} earlier lines not replayed"),
                    Style::new().add_modifier(Modifier::DIM),
                )),
            );
        }
        lines.push(Line::raw(""));
        terminal.insert_before(lines.len() as u16, |buf| {
            Paragraph::new(lines).render(buf.area, buf);
        })?;
        Ok(())
    }

    /// `true` to leave.
    async fn key(&mut self, key: KeyEvent) -> Result<bool> {
        match self.input.key(key) {
            Action::Submit(line) if !line.trim().is_empty() => {
                if let Some(command) = Command::parse(&line) {
                    return self.command(command).await;
                }
                self.transcript.push(Item::Prompt { text: line.clone() });
                self.transcript.push(Item::Blank);
                self.transcript.start();
                self.turn = Some(self.crab.spawn_turn(&line));
            }
            Action::Submit(_) => {}
            // A turn walks away; an idle Ctrl+C only clears the line.
            Action::Interrupt => match self.turn.take() {
                Some(_) => self.transcript.finish(),
                None => self.input.clear(),
            },
            Action::Eof => return Ok(true),
            Action::Noop => {}
        }
        Ok(false)
    }

    async fn command(&mut self, command: Command) -> Result<bool> {
        match command {
            Command::Exit => return Ok(true),
            Command::Help => {
                for (name, about) in HELP {
                    self.transcript.push(Item::Text {
                        md: format!("`{name}` — {about}"),
                        marker: false,
                    });
                }
                self.transcript.push(Item::Blank);
            }
            Command::Reconnect => {
                // A turn against the old engine has nowhere to land.
                self.turn = None;
                self.transcript.finish();
                match self.crab.reconnect().await {
                    Ok(()) => self.transcript.push(Item::Text {
                        md: "reconnected".to_owned(),
                        marker: true,
                    }),
                    Err(e) => self.transcript.push(Item::Error {
                        text: e.to_string(),
                    }),
                }
                self.transcript.push(Item::Blank);
            }
            Command::Compact => self.compact(false).await,
            Command::Unknown(name) => {
                self.transcript.push(Item::Error {
                    text: format!("no command /{name} — /help lists them"),
                });
                self.transcript.push(Item::Blank);
            }
        }
        Ok(false)
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
            Chunk::ContextUsage(context) => {
                self.tokens = context.usage.map(|usage| usage.prompt_tokens)
            }
            Chunk::ToolResult(result) => self.transcript.push_tool_result(&result.output),
            Chunk::ToolsComplete(_) => self.transcript.push_tool_done(),
            Chunk::End(end) if !end.error.is_empty() => self.fail(end.error),
            Chunk::End(_) => {
                self.transcript.finish();
                self.turn = None;
                self.compact_due = self.tokens.is_some_and(|used| self.crab.overflows(used));
            }
            _ => {}
        }
    }

    /// Summarize the conversation and work from the summary.
    ///
    /// `automatic` when the last turn left no room for the next one, in
    /// which case it says so — a conversation that rewrites itself
    /// without a word looks like one that lost its memory.
    async fn compact(&mut self, automatic: bool) {
        self.turn = None;
        self.transcript.finish();
        if automatic {
            self.transcript.push(Item::Text {
                md: "*the next turn would not fit — summarizing*".to_owned(),
                marker: false,
            });
        }
        match self.crab.compact().await {
            Ok(summary) => {
                self.transcript.push(Item::Text {
                    md: summary,
                    marker: true,
                });
                self.transcript.push(Item::Blank);
                self.transcript.push(Item::Text {
                    md: "*the conversation above is now this summary*".to_owned(),
                    marker: false,
                });
                self.tokens = None;
            }
            Err(e) => self.transcript.push(Item::Error {
                text: e.to_string(),
            }),
        }
        self.transcript.push(Item::Blank);
    }

    /// Say what went wrong and stay. A failed turn is not a reason to
    /// take the session down — the endpoint it could not reach is often
    /// back by the next one, and `/reconnect` is there for when it is not.
    fn fail(&mut self, error: String) {
        self.transcript.finish();
        self.transcript.push(Item::Error { text: error });
        self.transcript.push(Item::Blank);
        self.turn = None;
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
        let context = match (self.tokens, self.crab.config.context_window) {
            (Some(tokens), Some(window)) => format!(
                "  ·  {:.1}k / {:.0}k",
                tokens as f64 / 1000.0,
                window as f64 / 1000.0
            ),
            (Some(tokens), None) => format!("  ·  {:.1}k context", tokens as f64 / 1000.0),
            _ => String::new(),
        };
        Line::from(vec![
            Span::styled(format!("  {state}"), dim),
            Span::styled(context, dim),
            Span::styled(format!("  ·  {}  ·  /help", self.crab.root.display()), dim),
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
