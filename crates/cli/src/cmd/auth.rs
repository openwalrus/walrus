//! Interactive TUI for configuring channel API tokens.

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{io::Stdout, time::Duration};
use toml_edit::{DocumentMut, Item, Table};

const PLATFORMS: [&str; 2] = ["Telegram", "Discord"];
const FIELD_LABELS: [&str; 2] = ["Bot Token", "Agent"];

/// Configure channel API tokens interactively.
#[derive(clap::Args, Debug)]
pub struct Auth;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Platform,
    Field,
}

struct AuthState {
    focus: Focus,
    platform: usize,
    field_index: usize,
    editing: bool,
    /// Cursor position as a **char index** (not byte offset).
    cursor: usize,
    values: [[String; 2]; 2],
    status: String,
}

impl AuthState {
    fn load() -> Result<Self> {
        let config_path = wcore::paths::CONFIG_DIR.join("walrus.toml");
        let mut values = [
            [String::new(), String::new()],
            [String::new(), String::new()],
        ];

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("cannot read {}", config_path.display()))?;
            let doc: DocumentMut = content
                .parse()
                .with_context(|| format!("invalid TOML in {}", config_path.display()))?;

            if let Some(channel) = doc.get("channel").and_then(|c| c.as_table()) {
                if let Some(tg) = channel.get("telegram").and_then(|t| t.as_table()) {
                    if let Some(v) = tg.get("bot").and_then(|v| v.as_str()) {
                        values[0][0] = v.to_string();
                    }
                    if let Some(v) = tg.get("agent").and_then(|v| v.as_str()) {
                        values[0][1] = v.to_string();
                    }
                }
                if let Some(dc) = channel.get("discord").and_then(|t| t.as_table()) {
                    if let Some(v) = dc.get("token").and_then(|v| v.as_str()) {
                        values[1][0] = v.to_string();
                    }
                    if let Some(v) = dc.get("agent").and_then(|v| v.as_str()) {
                        values[1][1] = v.to_string();
                    }
                }
            }
        }

        Ok(Self {
            focus: Focus::Platform,
            platform: 0,
            field_index: 0,
            editing: false,
            cursor: 0,
            values,
            status: String::from("Ready"),
        })
    }

    fn save(&mut self) -> Result<()> {
        let config_path = wcore::paths::CONFIG_DIR.join("walrus.toml");
        std::fs::create_dir_all(&*wcore::paths::CONFIG_DIR)
            .with_context(|| format!("cannot create {}", wcore::paths::CONFIG_DIR.display()))?;

        let content = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .with_context(|| format!("cannot read {}", config_path.display()))?
        } else {
            String::new()
        };

        let mut doc: DocumentMut = content
            .parse()
            .with_context(|| format!("invalid TOML in {}", config_path.display()))?;

        let channel = doc
            .entry("channel")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .context("[channel] is not a table")?;

        // Telegram
        if self.values[0][0].is_empty() && self.values[0][1].is_empty() {
            channel.remove("telegram");
        } else {
            let tg = channel
                .entry("telegram")
                .or_insert(Item::Table(Table::new()))
                .as_table_mut()
                .context("[channel.telegram] is not a table")?;
            tg.insert("bot", toml_edit::value(&self.values[0][0]));
            if self.values[0][1].is_empty() {
                tg.remove("agent");
            } else {
                tg.insert("agent", toml_edit::value(&self.values[0][1]));
            }
        }

        // Discord
        if self.values[1][0].is_empty() && self.values[1][1].is_empty() {
            channel.remove("discord");
        } else {
            let dc = channel
                .entry("discord")
                .or_insert(Item::Table(Table::new()))
                .as_table_mut()
                .context("[channel.discord] is not a table")?;
            dc.insert("token", toml_edit::value(&self.values[1][0]));
            if self.values[1][1].is_empty() {
                dc.remove("agent");
            } else {
                dc.insert("agent", toml_edit::value(&self.values[1][1]));
            }
        }

        // Remove empty [channel] table
        if channel.is_empty() {
            doc.remove("channel");
        }

        std::fs::write(&config_path, doc.to_string())
            .with_context(|| format!("failed to write {}", config_path.display()))?;

        self.status = String::from("Saved!");
        Ok(())
    }

    fn current_value(&self) -> &str {
        &self.values[self.platform][self.field_index]
    }
}

/// Convert a char index to a byte offset within a string.
fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

impl Auth {
    pub fn run(self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        // Restore terminal on panic.
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
            original_hook(info);
        }));

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let mut state = AuthState::load()?;
        let result = run_loop(&mut terminal, &mut state);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut AuthState,
) -> Result<()> {
    loop {
        terminal.draw(|frame| render(frame, state))?;
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
            && handle_key(key, state)?
        {
            return Ok(());
        }
    }
}

/// Returns true if the TUI should quit.
fn handle_key(key: event::KeyEvent, state: &mut AuthState) -> Result<bool> {
    // Ctrl+S saves from anywhere
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        if let Err(e) = state.save() {
            state.status = format!("Error: {e}");
        }
        return Ok(false);
    }

    // Ctrl+C quits from anywhere
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(true);
    }

    if state.editing {
        return Ok(handle_editing(key, state));
    }

    match state.focus {
        Focus::Platform => match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Up | KeyCode::Char('k') => {
                state.platform = state.platform.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.platform < PLATFORMS.len() - 1 {
                    state.platform += 1;
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                state.focus = Focus::Field;
                state.field_index = 0;
            }
            KeyCode::Tab => {
                state.platform = (state.platform + 1) % PLATFORMS.len();
            }
            _ => {}
        },
        Focus::Field => match key.code {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                state.focus = Focus::Platform;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.field_index = state.field_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.field_index < FIELD_LABELS.len() - 1 {
                    state.field_index += 1;
                }
            }
            KeyCode::Enter => {
                state.editing = true;
                state.cursor = state.current_value().chars().count();
            }
            KeyCode::Tab => {
                state.field_index = (state.field_index + 1) % FIELD_LABELS.len();
            }
            _ => {}
        },
    }

    Ok(false)
}

/// Returns true if the TUI should quit.
fn handle_editing(key: event::KeyEvent, state: &mut AuthState) -> bool {
    let cursor = state.cursor;
    let val = &mut state.values[state.platform][state.field_index];
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            state.editing = false;
        }
        KeyCode::Backspace => {
            if cursor > 0 {
                let byte_pos = char_to_byte(val, cursor - 1);
                val.remove(byte_pos);
                state.cursor -= 1;
            }
        }
        KeyCode::Delete => {
            let char_count = val.chars().count();
            if cursor < char_count {
                let byte_pos = char_to_byte(val, cursor);
                val.remove(byte_pos);
            }
        }
        KeyCode::Left => {
            state.cursor = cursor.saturating_sub(1);
        }
        KeyCode::Right => {
            let char_count = val.chars().count();
            if cursor < char_count {
                state.cursor = cursor + 1;
            }
        }
        KeyCode::Home => {
            state.cursor = 0;
        }
        KeyCode::End => {
            state.cursor = val.chars().count();
        }
        KeyCode::Char(c) => {
            let byte_pos = char_to_byte(val, cursor);
            val.insert(byte_pos, c);
            state.cursor = cursor + 1;
        }
        _ => {}
    }
    false
}

fn render(frame: &mut Frame, state: &AuthState) {
    let area = frame.area();

    // Outer block
    let outer = Block::default()
        .title(" Walrus Auth ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Main layout: content area + status bar
    let vert = Layout::vertical([Constraint::Min(4), Constraint::Length(2)]).split(inner);
    let content_area = vert[0];
    let status_area = vert[1];

    // Horizontal split: platform list (30%) | fields (70%)
    let horiz = Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(content_area);
    let platform_area = horiz[0];
    let field_area = horiz[1];

    render_platforms(frame, state, platform_area);
    render_fields(frame, state, field_area);
    render_status(frame, state, status_area);
}

fn render_platforms(frame: &mut Frame, state: &AuthState, area: Rect) {
    let block = Block::default()
        .title(" Platform ")
        .borders(Borders::ALL)
        .border_style(if state.focus == Focus::Platform {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let lines: Vec<Line> = PLATFORMS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let marker = if i == state.platform { "> " } else { "  " };
            let style = if i == state.platform {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(format!("{marker}{name}"), style))
        })
        .collect();

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_fields(frame: &mut Frame, state: &AuthState, area: Rect) {
    let platform_name = PLATFORMS[state.platform];
    let block = Block::default()
        .title(format!(" {platform_name} "))
        .borders(Borders::ALL)
        .border_style(if state.focus == Focus::Field {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let field_rows =
        Layout::vertical(FIELD_LABELS.iter().map(|_| Constraint::Length(1))).split(inner);

    for (i, &label) in FIELD_LABELS.iter().enumerate() {
        let value = &state.values[state.platform][i];
        let is_selected = state.focus == Focus::Field && state.field_index == i;
        let is_editing = is_selected && state.editing;

        let label_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let display_value = if is_editing {
            // Show cursor in editing mode — insert at char boundary
            let byte_pos = char_to_byte(value, state.cursor);
            let mut s = value.clone();
            s.insert(byte_pos, '|');
            s
        } else if value.is_empty() {
            String::from("(empty)")
        } else if label == "Bot Token" {
            mask_token(value)
        } else {
            value.clone()
        };

        let value_style = if is_editing {
            Style::default().fg(Color::Green)
        } else if is_selected {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let line = Line::from(vec![
            Span::styled(format!("{label:>10}: "), label_style),
            Span::styled(display_value, value_style),
        ]);

        frame.render_widget(Paragraph::new(line), field_rows[i]);
    }
}

fn render_status(frame: &mut Frame, state: &AuthState, area: Rect) {
    let help = Line::from(vec![
        Span::styled(" Ctrl+S ", Style::default().fg(Color::Cyan)),
        Span::raw("Save  "),
        Span::styled("Esc ", Style::default().fg(Color::Cyan)),
        Span::raw("Back  "),
        Span::styled("Enter ", Style::default().fg(Color::Cyan)),
        Span::raw("Edit  "),
        Span::styled("q ", Style::default().fg(Color::Cyan)),
        Span::raw("Quit  "),
        Span::styled("| ", Style::default().fg(Color::DarkGray)),
        Span::styled(&state.status, Style::default().fg(Color::Green)),
    ]);
    frame.render_widget(Paragraph::new(help), area);
}

/// Mask a token for display — show first 4 and last 4 ASCII chars.
fn mask_token(token: &str) -> String {
    let chars: Vec<char> = token.chars().collect();
    if chars.len() <= 8 {
        "*".repeat(chars.len())
    } else {
        let head: String = chars[..4].iter().collect();
        let tail: String = chars[chars.len() - 4..].iter().collect();
        format!("{head}{}{tail}", "*".repeat(chars.len() - 8))
    }
}
