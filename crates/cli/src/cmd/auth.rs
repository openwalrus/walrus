//! Interactive TUI for configuring channel entries.

use anyhow::{Context, Result};
use channel::ChannelType;
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
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::{io::Stdout, time::Duration};
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table};

const FIELD_LABELS: [&str; 3] = ["Type", "Token", "Agent"];

/// Configure channel entries interactively.
#[derive(clap::Args, Debug)]
pub struct Auth;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    List,
    Field,
    /// Inline type selector overlay — index into `ChannelType::VARIANTS`.
    TypeSelect(usize),
}

struct ChannelEntryState {
    channel_type: ChannelType,
    token: String,
    agent: String,
}

struct AuthState {
    focus: Focus,
    /// Index into `entries`, or `entries.len()` for the "+ Add channel" row.
    selected: usize,
    field_index: usize,
    editing: bool,
    cursor: usize,
    entries: Vec<ChannelEntryState>,
    status: String,
}

impl AuthState {
    fn load() -> Result<Self> {
        let config_path = wcore::paths::CONFIG_DIR.join("walrus.toml");
        let mut entries = Vec::new();

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("cannot read {}", config_path.display()))?;
            let doc: DocumentMut = content
                .parse()
                .with_context(|| format!("invalid TOML in {}", config_path.display()))?;

            if let Some(channels) = doc.get("channel").and_then(|c| c.as_array_of_tables()) {
                for table in channels.iter() {
                    let channel_type = match table.get("type").and_then(|v| v.as_str()) {
                        Some("telegram") => ChannelType::Telegram,
                        Some("discord") => ChannelType::Discord,
                        _ => continue,
                    };
                    let token = table
                        .get("token")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let agent = table
                        .get("agent")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    entries.push(ChannelEntryState {
                        channel_type,
                        token,
                        agent,
                    });
                }
            }
        }

        Ok(Self {
            focus: Focus::List,
            selected: 0,
            field_index: 0,
            editing: false,
            cursor: 0,
            entries,
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

        doc.remove("channel");

        if !self.entries.is_empty() {
            let mut arr = ArrayOfTables::new();
            for entry in &self.entries {
                let mut table = Table::new();
                table.insert(
                    "type",
                    toml_edit::value(entry.channel_type.to_string().to_lowercase()),
                );
                table.insert("token", toml_edit::value(&entry.token));
                if !entry.agent.is_empty() {
                    table.insert("agent", toml_edit::value(&entry.agent));
                }
                arr.push(table);
            }
            doc.insert("channel", Item::ArrayOfTables(arr));
        }

        std::fs::write(&config_path, doc.to_string())
            .with_context(|| format!("failed to write {}", config_path.display()))?;

        self.status = String::from("Saved!");
        Ok(())
    }

    fn on_add_row(&self) -> bool {
        self.selected == self.entries.len()
    }

    fn current_text_field(&self) -> &str {
        let entry = &self.entries[self.selected];
        match self.field_index {
            1 => &entry.token,
            2 => &entry.agent,
            _ => "",
        }
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
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        if let Err(e) = state.save() {
            state.status = format!("Error: {e}");
        }
        return Ok(false);
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(true);
    }

    if state.editing {
        return Ok(handle_editing(key, state));
    }

    match state.focus {
        Focus::List => handle_list_key(key, state),
        Focus::Field => handle_field_key(key, state),
        Focus::TypeSelect(sel) => handle_type_select_key(key, state, sel),
    }
}

fn handle_list_key(key: event::KeyEvent, state: &mut AuthState) -> Result<bool> {
    let row_count = state.entries.len() + 1; // entries + "Add" row
    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected = state.selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.selected < row_count - 1 {
                state.selected += 1;
            }
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            if state.on_add_row() {
                state.entries.push(ChannelEntryState {
                    channel_type: ChannelType::VARIANTS[0],
                    token: String::new(),
                    agent: String::new(),
                });
                state.selected = state.entries.len() - 1;
            }
            if !state.entries.is_empty() {
                state.focus = Focus::Field;
                state.field_index = 0;
            }
        }
        KeyCode::Char('a') => {
            state.entries.push(ChannelEntryState {
                channel_type: ChannelType::VARIANTS[0],
                token: String::new(),
                agent: String::new(),
            });
            state.selected = state.entries.len() - 1;
            state.focus = Focus::Field;
            state.field_index = 0;
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if !state.on_add_row() && !state.entries.is_empty() {
                state.entries.remove(state.selected);
                if state.selected >= state.entries.len() && !state.entries.is_empty() {
                    state.selected = state.entries.len() - 1;
                }
            }
        }
        KeyCode::Tab => {
            state.selected = (state.selected + 1) % row_count;
        }
        _ => {}
    }
    Ok(false)
}

fn handle_field_key(key: event::KeyEvent, state: &mut AuthState) -> Result<bool> {
    match key.code {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            state.focus = Focus::List;
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
            if state.field_index == 0 {
                // Open type selector — pre-select current type
                let current = state.entries[state.selected].channel_type;
                let idx = ChannelType::VARIANTS
                    .iter()
                    .position(|&v| v == current)
                    .unwrap_or(0);
                state.focus = Focus::TypeSelect(idx);
            } else {
                state.editing = true;
                state.cursor = state.current_text_field().chars().count();
            }
        }
        KeyCode::Tab => {
            state.field_index = (state.field_index + 1) % FIELD_LABELS.len();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_type_select_key(key: event::KeyEvent, state: &mut AuthState, sel: usize) -> Result<bool> {
    let variant_count = ChannelType::VARIANTS.len();
    match key.code {
        KeyCode::Esc => {
            state.focus = Focus::Field;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.focus = Focus::TypeSelect(sel.saturating_sub(1));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if sel < variant_count - 1 {
                state.focus = Focus::TypeSelect(sel + 1);
            }
        }
        KeyCode::Enter => {
            state.entries[state.selected].channel_type = ChannelType::VARIANTS[sel];
            state.focus = Focus::Field;
        }
        _ => {}
    }
    Ok(false)
}

/// Returns true if the TUI should quit.
fn handle_editing(key: event::KeyEvent, state: &mut AuthState) -> bool {
    let cursor = state.cursor;
    let entry = &mut state.entries[state.selected];
    let val = match state.field_index {
        1 => &mut entry.token,
        2 => &mut entry.agent,
        _ => return false,
    };

    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            state.editing = false;
        }
        KeyCode::Backspace => {
            if cursor > 0 {
                let start = char_to_byte(val, cursor - 1);
                let end = char_to_byte(val, cursor);
                val.drain(start..end);
                state.cursor -= 1;
            }
        }
        KeyCode::Delete => {
            let char_count = val.chars().count();
            if cursor < char_count {
                let start = char_to_byte(val, cursor);
                let end = char_to_byte(val, cursor + 1);
                val.drain(start..end);
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

// ── Rendering ────────────────────────────────────────────────────────

fn render(frame: &mut Frame, state: &AuthState) {
    let area = frame.area();

    let outer = Block::default()
        .title(" Walrus Auth ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let vert = Layout::vertical([Constraint::Min(4), Constraint::Length(2)]).split(inner);
    let content_area = vert[0];
    let status_area = vert[1];

    let horiz = Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(content_area);
    let list_area = horiz[0];
    let field_area = horiz[1];

    render_list(frame, state, list_area);
    render_fields(frame, state, field_area);
    render_status(frame, state, status_area);

    // Type selector overlay — rendered last so it draws on top.
    if let Focus::TypeSelect(sel) = state.focus {
        render_type_selector(frame, field_area, sel);
    }
}

/// Build display names for the channel list: `Type:agent` with `-N` suffix
/// when multiple entries share the same (type, agent) pair.
fn channel_display_names(entries: &[ChannelEntryState]) -> Vec<String> {
    // Count occurrences of each (type, agent) pair.
    let mut counts: std::collections::HashMap<(ChannelType, &str), usize> =
        std::collections::HashMap::new();
    for e in entries {
        let agent = if e.agent.is_empty() {
            "default"
        } else {
            &e.agent
        };
        *counts.entry((e.channel_type, agent)).or_default() += 1;
    }

    // Assign sequential numbers to duplicates.
    let mut seen: std::collections::HashMap<(ChannelType, &str), usize> =
        std::collections::HashMap::new();
    entries
        .iter()
        .map(|e| {
            let agent = if e.agent.is_empty() {
                "default"
            } else {
                &e.agent
            };
            let key = (e.channel_type, agent);
            let total = counts[&key];
            let seq = seen.entry(key).or_default();
            *seq += 1;
            let type_lower = e.channel_type.to_string().to_lowercase();
            if total > 1 {
                format!("{type_lower}:{agent}-{seq}")
            } else {
                format!("{type_lower}:{agent}")
            }
        })
        .collect()
}

fn render_list(frame: &mut Frame, state: &AuthState, area: Rect) {
    let block = Block::default()
        .title(" Channels ")
        .borders(Borders::ALL)
        .border_style(if state.focus == Focus::List {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let display_names = channel_display_names(&state.entries);
    let mut lines: Vec<Line> = display_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let marker = if i == state.selected { "> " } else { "  " };
            let text = format!("{marker}{name}");
            let style = if i == state.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(text, style))
        })
        .collect();

    // "+ Add channel" row
    let add_marker = if state.on_add_row() { "> " } else { "  " };
    let add_style = if state.on_add_row() {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(Span::styled(
        format!("{add_marker}+ Add channel"),
        add_style,
    )));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_fields(frame: &mut Frame, state: &AuthState, area: Rect) {
    if state.on_add_row() || state.entries.is_empty() {
        let block = Block::default()
            .title(" Channel ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let hint = Paragraph::new(Line::from(Span::styled(
            "  Select a channel or add one",
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        frame.render_widget(hint, area);
        return;
    }

    let entry = &state.entries[state.selected];
    let block = Block::default()
        .title(format!(" {} ", entry.channel_type))
        .borders(Borders::ALL)
        .border_style(
            if matches!(state.focus, Focus::Field | Focus::TypeSelect(_)) {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let field_rows =
        Layout::vertical(FIELD_LABELS.iter().map(|_| Constraint::Length(1))).split(inner);

    for (i, &label) in FIELD_LABELS.iter().enumerate() {
        let is_selected =
            matches!(state.focus, Focus::Field | Focus::TypeSelect(_)) && state.field_index == i;
        let is_editing = is_selected && state.editing;

        let label_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let label_span = Span::styled(format!("{label:>10}: "), label_style);

        let line = match i {
            // Type field
            0 => {
                let type_name = entry.channel_type.to_string();
                if is_selected {
                    Line::from(vec![
                        label_span,
                        Span::styled(type_name, Style::default().fg(Color::Cyan)),
                        Span::styled("  (Enter to select)", Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    Line::from(vec![
                        label_span,
                        Span::styled(type_name, Style::default().fg(Color::DarkGray)),
                    ])
                }
            }
            // Token field
            1 => {
                if is_editing {
                    let byte_pos = char_to_byte(&entry.token, state.cursor);
                    let mut s = entry.token.clone();
                    s.insert(byte_pos, '|');
                    Line::from(vec![
                        label_span,
                        Span::styled(s, Style::default().fg(Color::Green)),
                    ])
                } else if entry.token.is_empty() {
                    Line::from(vec![
                        label_span,
                        Span::styled(
                            entry.channel_type.token_hint(),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ])
                } else {
                    let style = if is_selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    Line::from(vec![
                        label_span,
                        Span::styled(mask_token(&entry.token), style),
                    ])
                }
            }
            // Agent field
            2 => {
                if is_editing {
                    let byte_pos = char_to_byte(&entry.agent, state.cursor);
                    let mut s = entry.agent.clone();
                    s.insert(byte_pos, '|');
                    Line::from(vec![
                        label_span,
                        Span::styled(s, Style::default().fg(Color::Green)),
                    ])
                } else if entry.agent.is_empty() {
                    Line::from(vec![
                        label_span,
                        Span::styled("walrus", Style::default().fg(Color::DarkGray)),
                        Span::styled(" (default)", Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    let style = if is_selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    Line::from(vec![label_span, Span::styled(entry.agent.as_str(), style)])
                }
            }
            _ => Line::default(),
        };

        frame.render_widget(Paragraph::new(line), field_rows[i]);
    }
}

/// Render the type selector as an overlay popup anchored to the field area.
fn render_type_selector(frame: &mut Frame, field_area: Rect, sel: usize) {
    let variants = ChannelType::VARIANTS;
    let height = variants.len() as u16 + 2; // +2 for borders
    let width = 20u16.min(field_area.width);

    // Position below the Type field row (offset y by 1 for the block border + 0 for first field).
    let popup = Rect {
        x: field_area.x + 12, // align with value column (after "      Type: ")
        y: field_area.y + 1,
        width,
        height: height.min(field_area.height),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let lines: Vec<Line> = variants
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let marker = if i == sel { "> " } else { "  " };
            let style = if i == sel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(format!("{marker}{v}"), style))
        })
        .collect();

    // Clear the area behind the popup, then draw it.
    frame.render_widget(Clear, popup);
    frame.render_widget(Paragraph::new(lines).block(block), popup);
}

fn render_status(frame: &mut Frame, state: &AuthState, area: Rect) {
    let help = Line::from(vec![
        Span::styled(" Ctrl+S ", Style::default().fg(Color::Cyan)),
        Span::raw("Save  "),
        Span::styled("a ", Style::default().fg(Color::Cyan)),
        Span::raw("Add  "),
        Span::styled("d ", Style::default().fg(Color::Cyan)),
        Span::raw("Delete  "),
        Span::styled("Enter ", Style::default().fg(Color::Cyan)),
        Span::raw("Edit  "),
        Span::styled("Esc ", Style::default().fg(Color::Cyan)),
        Span::raw("Back  "),
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
