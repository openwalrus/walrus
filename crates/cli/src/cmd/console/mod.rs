//! Interactive TUI for managing sessions.

use crate::repl::runner::Runner;
use crate::tui;
use anyhow::Result;
use clap::Args;
use crossterm::event::{KeyCode, KeyModifiers};
use events::EventEntry;
use futures_util::StreamExt;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Tabs},
};
use std::collections::VecDeque;
use tokio::sync::mpsc;
use wcore::protocol::message::{AgentEventMsg, SessionInfo};

mod events;
mod sessions;

use events::render_events;
use sessions::render_sessions;

/// Interactive console for sessions.
#[derive(Args, Debug)]
pub struct Console;

impl Console {
    pub async fn run(self, runner: Runner) -> Result<()> {
        // Spawn background event subscription task.
        let (event_tx, event_rx) = mpsc::unbounded_channel::<AgentEventMsg>();
        let socket_path = wcore::paths::SOCKET_PATH.to_path_buf();
        tokio::spawn(async move {
            let Ok(mut sub_runner) = Runner::connect(&socket_path).await else {
                return;
            };
            let stream = sub_runner.subscribe_events();
            tokio::pin!(stream);
            while let Some(Ok(msg)) = stream.next().await {
                if event_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        // Show TUI immediately with empty sessions, load in background.
        let mut terminal = tui::setup()?;
        let mut state = ConsoleState {
            sessions: Vec::new(),
            selected: 0,
            status: String::from("Loading..."),
            runner,
            tab: Tab::Sessions,
            events: VecDeque::new(),
            event_rx,
            event_scroll: 0,
        };

        let mut idle_ticks: u8 = 0;
        let mut needs_refresh = true;
        let result = loop {
            // Drain any pending events.
            while let Ok(msg) = state.event_rx.try_recv() {
                let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
                state.events.push_back(EventEntry { timestamp, msg });
                if state.events.len() > 500 {
                    state.events.pop_front();
                }
            }

            terminal.draw(|frame| render(frame, &state))?;
            if let Some(key) = tui::poll_key()? {
                idle_ticks = 0;
                if let Some(result) = handle_key(key, &mut state).await? {
                    break result;
                }
            } else {
                idle_ticks = idle_ticks.saturating_add(1);
                if needs_refresh || idle_ticks >= 4 {
                    idle_ticks = 0;
                    needs_refresh = false;
                    // Non-blocking refresh: timeout after 500ms to avoid freezing.
                    let fut = state.runner.list_sessions();
                    match tokio::time::timeout(std::time::Duration::from_millis(500), fut).await {
                        Ok(Ok(sessions)) => {
                            state.sessions = sessions;
                            if state.selected >= state.sessions.len() {
                                state.selected = state.sessions.len().saturating_sub(1);
                            }
                            state.status = String::from("Ready");
                        }
                        Ok(Err(_)) => {
                            state.status = String::from("Error refreshing");
                        }
                        Err(_) => {
                            // Timeout — don't block the TUI.
                        }
                    }
                }
            }
        };

        tui::teardown(&mut terminal)?;
        result
    }
}

// ── State ────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Sessions,
    Events,
}

pub(crate) struct ConsoleState {
    pub(crate) sessions: Vec<SessionInfo>,
    pub(crate) selected: usize,
    pub(crate) status: String,
    pub(crate) runner: Runner,
    tab: Tab,
    events: VecDeque<EventEntry>,
    event_rx: mpsc::UnboundedReceiver<AgentEventMsg>,
    event_scroll: usize,
}

impl ConsoleState {}

// ── Key handling ────────────────────────────────────────────────────

async fn handle_key(
    key: crossterm::event::KeyEvent,
    state: &mut ConsoleState,
) -> Result<Option<Result<()>>> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(Some(Ok(())));
    }

    match key.code {
        KeyCode::Char('q') => return Ok(Some(Ok(()))),
        KeyCode::Tab => {
            state.tab = match state.tab {
                Tab::Sessions => Tab::Events,
                Tab::Events => Tab::Sessions,
            };
        }
        _ => match state.tab {
            Tab::Sessions => handle_sessions_key(key.code, state).await,
            Tab::Events => handle_events_key(key.code, state),
        },
    }
    Ok(None)
}

async fn handle_sessions_key(code: KeyCode, state: &mut ConsoleState) {
    let timeout = std::time::Duration::from_millis(500);
    let len = state.sessions.len();
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected = state.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if len > 0 && state.selected < len - 1 {
                state.selected += 1;
            }
        }
        KeyCode::Char('r') => {
            if let Ok(Ok(sessions)) =
                tokio::time::timeout(timeout, state.runner.list_sessions()).await
            {
                state.sessions = sessions;
                state.status = String::from("Refreshed");
            }
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if let Some(s) = state.sessions.get(state.selected) {
                let id = s.id;
                match tokio::time::timeout(timeout, state.runner.kill_session(id)).await {
                    Ok(Ok(true)) => state.status = format!("Session {id} killed"),
                    Ok(Ok(false)) => state.status = format!("Session {id} not found"),
                    Ok(Err(e)) => state.status = format!("Error: {e}"),
                    Err(_) => state.status = String::from("Timeout"),
                }
            }
            if let Ok(Ok(sessions)) =
                tokio::time::timeout(timeout, state.runner.list_sessions()).await
            {
                state.sessions = sessions;
            }
        }
        _ => {}
    }
}

fn handle_events_key(code: KeyCode, state: &mut ConsoleState) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            state.event_scroll = state.event_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !state.events.is_empty() {
                state.event_scroll = (state.event_scroll + 1).min(state.events.len() - 1);
            }
        }
        _ => {}
    }
}

// ── Render ──────────────────────────────────────────────────────────

const TAB_TITLES: &[&str] = &["Sessions", "Events"];

fn render(frame: &mut Frame, state: &ConsoleState) {
    let chunks = Layout::default()
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    // Tab bar
    let tab_idx = match state.tab {
        Tab::Sessions => 0,
        Tab::Events => 1,
    };
    let tabs = Tabs::new(TAB_TITLES.iter().map(|t| Line::from(*t)))
        .select(tab_idx)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" | ");
    frame.render_widget(tabs, chunks[0]);

    // Content
    match state.tab {
        Tab::Sessions => render_sessions(frame, state, chunks[1]),
        Tab::Events => {
            let events: Vec<&EventEntry> = state.events.iter().collect();
            render_events(frame, &events, state.event_scroll, chunks[1]);
        }
    }

    // Help bar
    render_help_bar(frame, state, chunks[2]);
}

fn render_help_bar(frame: &mut Frame, state: &ConsoleState, area: Rect) {
    let key_style = Style::default().fg(Color::Cyan);
    let mut spans = vec![Span::styled(" j/k ", key_style), Span::raw("Navigate  ")];

    if state.tab == Tab::Sessions {
        spans.extend([
            Span::styled("d ", key_style),
            Span::raw("Kill  "),
            Span::styled("r ", key_style),
            Span::raw("Refresh  "),
        ]);
    }

    spans.extend([
        Span::styled("Tab ", key_style),
        Span::raw("Switch  "),
        Span::styled("q ", key_style),
        Span::raw("Quit  "),
        Span::styled(
            format!(" {} ", state.status),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
