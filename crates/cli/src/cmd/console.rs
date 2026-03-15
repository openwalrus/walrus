//! Interactive TUI for managing sessions and tasks.

use crate::repl::runner::Runner;
use crate::tui::{self, border_focused, format_duration, handle_text_input};
use anyhow::Result;
use clap::Args;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
};
use wcore::protocol::message::{SessionInfo, TaskInfo};

/// Interactive console for sessions and tasks.
#[derive(Args, Debug)]
pub struct Console;

impl Console {
    pub async fn run(self, mut runner: Runner) -> Result<()> {
        let sessions = runner.list_sessions().await.unwrap_or_default();
        let tasks = runner.list_tasks().await.unwrap_or_default();

        let mut terminal = tui::setup()?;
        let mut state = ConsoleState {
            tab: Tab::Sessions,
            focus: Focus::List,
            sessions,
            tasks,
            selected: 0,
            cursor: 0,
            edit_buf: String::new(),
            status: String::from("Ready"),
            runner,
        };

        let result = loop {
            terminal.draw(|frame| render(frame, &state))?;
            if let Some(key) = tui::poll_key()?
                && let Some(result) = handle_key(key, &mut state).await?
            {
                break result;
            }
        };

        tui::teardown(&mut terminal)?;
        result
    }
}

// ── Tabs ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Sessions,
    Tasks,
}

const TAB_TITLES: &[&str] = &["Sessions", "Tasks"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    List,
    Approve,
}

// ── State ────────────────────────────────────────────────────────────

struct ConsoleState {
    tab: Tab,
    focus: Focus,
    sessions: Vec<SessionInfo>,
    tasks: Vec<TaskInfo>,
    selected: usize,
    cursor: usize,
    edit_buf: String,
    status: String,
    runner: Runner,
}

impl ConsoleState {
    async fn refresh(&mut self) {
        match self.tab {
            Tab::Sessions => {
                self.sessions = self.runner.list_sessions().await.unwrap_or_default();
                if self.selected >= self.sessions.len() {
                    self.selected = self.sessions.len().saturating_sub(1);
                }
            }
            Tab::Tasks => {
                self.tasks = self.runner.list_tasks().await.unwrap_or_default();
                if self.selected >= self.tasks.len() {
                    self.selected = self.tasks.len().saturating_sub(1);
                }
            }
        }
    }

    fn list_len(&self) -> usize {
        match self.tab {
            Tab::Sessions => self.sessions.len(),
            Tab::Tasks => self.tasks.len(),
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

    if key.code == KeyCode::Tab && state.focus == Focus::List {
        state.tab = match state.tab {
            Tab::Sessions => Tab::Tasks,
            Tab::Tasks => Tab::Sessions,
        };
        state.selected = 0;
        state.refresh().await;
        return Ok(None);
    }

    match state.focus {
        Focus::List => handle_list(key, state).await,
        Focus::Approve => {
            handle_approve_input(key, state).await;
            Ok(None)
        }
    }
}

async fn handle_list(
    key: crossterm::event::KeyEvent,
    state: &mut ConsoleState,
) -> Result<Option<Result<()>>> {
    let len = state.list_len();
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
            match state.tab {
                Tab::Sessions => {
                    if let Some(s) = state.sessions.get(state.selected) {
                        let id = s.id;
                        match state.runner.kill_session(id).await {
                            Ok(true) => state.status = format!("Session {id} killed"),
                            Ok(false) => state.status = format!("Session {id} not found"),
                            Err(e) => state.status = format!("Error: {e}"),
                        }
                    }
                }
                Tab::Tasks => {
                    if let Some(t) = state.tasks.get(state.selected) {
                        let id = t.id;
                        match state.runner.kill_task(id).await {
                            Ok(true) => state.status = format!("Task {id} killed"),
                            Ok(false) => state.status = format!("Task {id} not found"),
                            Err(e) => state.status = format!("Error: {e}"),
                        }
                    }
                }
            }
            state.refresh().await;
        }
        KeyCode::Char('a') => {
            // Approve a blocked task.
            if state.tab == Tab::Tasks {
                if let Some(t) = state.tasks.get(state.selected)
                    && t.blocked_on.is_some()
                {
                    state.focus = Focus::Approve;
                    state.edit_buf.clear();
                    state.cursor = 0;
                } else {
                    state.status = String::from("Task is not blocked");
                }
            }
        }
        _ => {}
    }
    Ok(None)
}

async fn handle_approve_input(key: crossterm::event::KeyEvent, state: &mut ConsoleState) {
    match key.code {
        KeyCode::Esc => {
            state.focus = Focus::List;
        }
        KeyCode::Enter => {
            if let Some(t) = state.tasks.get(state.selected) {
                let id = t.id;
                let response = state.edit_buf.clone();
                match state.runner.approve_task(id, response).await {
                    Ok(true) => state.status = format!("Task {id} approved"),
                    Ok(false) => state.status = format!("Task {id} not blocked"),
                    Err(e) => state.status = format!("Error: {e}"),
                }
            }
            state.focus = Focus::List;
            state.refresh().await;
        }
        _ => handle_text_input(key.code, &mut state.edit_buf, &mut state.cursor),
    }
}

// ── Rendering ───────────────────────────────────────────────────────

fn render(frame: &mut Frame, state: &ConsoleState) {
    let area = frame.area();

    let outer = Block::default()
        .title(" Walrus Console ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(4),
        Constraint::Length(2),
    ])
    .split(inner);

    // Tab bar.
    let tab_idx = match state.tab {
        Tab::Sessions => 0,
        Tab::Tasks => 1,
    };
    let tabs = Tabs::new(TAB_TITLES.iter().map(|t| Line::from(*t)))
        .select(tab_idx)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" | ");
    frame.render_widget(tabs, vert[0]);

    match state.tab {
        Tab::Sessions => render_sessions(frame, state, vert[1]),
        Tab::Tasks => render_tasks(frame, state, vert[1]),
    }

    render_status(frame, state, vert[2]);
}

fn render_sessions(frame: &mut Frame, state: &ConsoleState, area: Rect) {
    let block = Block::default()
        .title(" Sessions ")
        .borders(Borders::ALL)
        .border_style(border_focused());

    if state.sessions.is_empty() {
        frame.render_widget(Paragraph::new("  No active sessions.").block(block), area);
        return;
    }

    let mut lines = vec![Line::from(vec![Span::styled(
        format!(
            "  {:<6} {:<16} {:<16} {:<8} {:<10}",
            "ID", "AGENT", "CREATED BY", "MSGS", "ALIVE"
        ),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )])];

    for (i, s) in state.sessions.iter().enumerate() {
        let is_selected = i == state.selected;
        let marker = if is_selected { "> " } else { "  " };
        let alive = format_duration(s.alive_secs);
        let text = format!(
            "{marker}{:<6} {:<16} {:<16} {:<8} {:<10}",
            s.id, s.agent, s.created_by, s.message_count, alive
        );
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(text, style)));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_tasks(frame: &mut Frame, state: &ConsoleState, area: Rect) {
    let horiz = if state.focus == Focus::Approve {
        Layout::vertical([Constraint::Min(4), Constraint::Length(3)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(4)]).split(area)
    };

    let block = Block::default()
        .title(" Tasks ")
        .borders(Borders::ALL)
        .border_style(border_focused());

    if state.tasks.is_empty() {
        frame.render_widget(Paragraph::new("  No active tasks.").block(block), horiz[0]);
    } else {
        let mut lines = vec![Line::from(vec![Span::styled(
            format!(
                "  {:<6} {:<16} {:<12} {:<10} {:<10}",
                "ID", "AGENT", "STATUS", "ALIVE", "TOKENS"
            ),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )])];

        for (i, t) in state.tasks.iter().enumerate() {
            let is_selected = i == state.selected;
            let marker = if is_selected { "> " } else { "  " };
            let alive = format_duration(t.alive_secs);
            let tokens = t.prompt_tokens + t.completion_tokens;
            let text = format!(
                "{marker}{:<6} {:<16} {:<12} {:<10} {:<10}",
                t.id, t.agent, t.status, alive, tokens
            );
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(text, style)));

            if let Some(q) = &t.blocked_on {
                let blocked_style = Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::ITALIC);
                lines.push(Line::from(Span::styled(
                    format!("         blocked: {q}"),
                    blocked_style,
                )));
            }
        }

        frame.render_widget(Paragraph::new(lines).block(block), horiz[0]);
    }

    // Approve input area.
    if state.focus == Focus::Approve && horiz.len() > 1 {
        let block = Block::default()
            .title(" Approve Response ")
            .borders(Borders::ALL)
            .border_style(border_focused());
        let inner = block.inner(horiz[1]);
        frame.render_widget(block, horiz[1]);

        let byte_pos = tui::char_to_byte(&state.edit_buf, state.cursor);
        let mut s = state.edit_buf.clone();
        s.insert(byte_pos, '|');
        frame.render_widget(
            Paragraph::new(Span::styled(s, Style::default().fg(Color::Green))),
            inner,
        );
    }
}

fn render_status(frame: &mut Frame, state: &ConsoleState, area: Rect) {
    let help = match state.focus {
        Focus::Approve => Line::from(vec![
            Span::styled(" Enter ", Style::default().fg(Color::Cyan)),
            Span::raw("Send  "),
            Span::styled("Esc ", Style::default().fg(Color::Cyan)),
            Span::raw("Cancel  "),
            Span::styled("| ", Style::default().fg(Color::DarkGray)),
            Span::styled(&state.status, Style::default().fg(Color::Green)),
        ]),
        Focus::List => {
            let mut spans = vec![
                Span::styled(" Tab ", Style::default().fg(Color::Cyan)),
                Span::raw("Switch  "),
                Span::styled("r ", Style::default().fg(Color::Cyan)),
                Span::raw("Refresh  "),
                Span::styled("d ", Style::default().fg(Color::Cyan)),
                Span::raw("Kill  "),
            ];
            if state.tab == Tab::Tasks {
                spans.push(Span::styled("a ", Style::default().fg(Color::Cyan)));
                spans.push(Span::raw("Approve  "));
            }
            spans.push(Span::styled("q ", Style::default().fg(Color::Cyan)));
            spans.push(Span::raw("Quit  "));
            spans.push(Span::styled("| ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                &state.status,
                Style::default().fg(Color::Green),
            ));
            Line::from(spans)
        }
    };
    frame.render_widget(Paragraph::new(help), area);
}
