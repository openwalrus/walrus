//! Interactive TUI for managing sessions.

use crate::repl::runner::Runner;
use crate::tui;
use anyhow::Result;
use clap::Args;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use sessions::render_sessions;
use wcore::protocol::message::SessionInfo;

mod sessions;

/// Interactive console for sessions.
#[derive(Args, Debug)]
pub struct Console;

impl Console {
    pub async fn run(self, mut runner: Runner) -> Result<()> {
        let sessions = runner.list_sessions().await.unwrap_or_default();

        let mut terminal = tui::setup()?;
        let mut state = ConsoleState {
            sessions,
            selected: 0,
            status: String::from("Ready"),
            runner,
        };

        let mut idle_ticks: u8 = 0;
        let result = loop {
            terminal.draw(|frame| render(frame, &state))?;
            if let Some(key) = tui::poll_key()? {
                idle_ticks = 0;
                if let Some(result) = handle_key(key, &mut state).await? {
                    break result;
                }
            } else {
                idle_ticks = idle_ticks.saturating_add(1);
                if idle_ticks >= 4 {
                    idle_ticks = 0;
                    state.refresh().await;
                }
            }
        };

        tui::teardown(&mut terminal)?;
        result
    }
}

// ── State ────────────────────────────────────────────────────────

pub(crate) struct ConsoleState {
    pub(crate) sessions: Vec<SessionInfo>,
    pub(crate) selected: usize,
    pub(crate) status: String,
    pub(crate) runner: Runner,
}

impl ConsoleState {
    pub(crate) async fn refresh(&mut self) {
        self.sessions = self.runner.list_sessions().await.unwrap_or_default();
        if self.selected >= self.sessions.len() {
            self.selected = self.sessions.len().saturating_sub(1);
        }
    }
}

// ── Key handling ────────────────────────────────────────────────────

async fn handle_key(
    key: crossterm::event::KeyEvent,
    state: &mut ConsoleState,
) -> Result<Option<Result<()>>> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(Some(Ok(())));
    }

    let len = state.sessions.len();
    match key.code {
        KeyCode::Char('q') => return Ok(Some(Ok(()))),
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected = state.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if len > 0 && state.selected < len - 1 {
                state.selected += 1;
            }
        }
        KeyCode::Char('r') => {
            state.refresh().await;
            state.status = String::from("Refreshed");
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if let Some(s) = state.sessions.get(state.selected) {
                let id = s.id;
                match state.runner.kill_session(id).await {
                    Ok(true) => state.status = format!("Session {id} killed"),
                    Ok(false) => state.status = format!("Session {id} not found"),
                    Err(e) => state.status = format!("Error: {e}"),
                }
            }
            state.refresh().await;
        }
        _ => {}
    }
    Ok(None)
}

// ── Render ──────────────────────────────────────────────────────────

fn render(frame: &mut Frame, state: &ConsoleState) {
    let chunks = Layout::default()
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    render_sessions(frame, state, chunks[0]);

    let status = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {} ", state.status),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(status, chunks[1]);
}
