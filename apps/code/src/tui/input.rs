//! The input box: a multi-line editor with history.
//!
//! It owns the bottom of the viewport and nothing else, so it is handed
//! a `Rect` and draws into it — where the box sits is the loop's
//! business.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::path::Path;

/// What a keystroke asked the loop to do.
pub enum Action {
    Submit(String),
    /// Ctrl+C — cancel the turn, or clear the line.
    Interrupt,
    /// Ctrl+D on an empty line.
    Eof,
    Noop,
}

/// Submitted lines, oldest first, with a cursor for Up/Down recall.
pub struct History {
    entries: Vec<String>,
    cursor: usize,
    /// What was typed before recall started, restored on the way back.
    stash: String,
}

impl History {
    pub fn load(path: &Path) -> Self {
        let entries: Vec<String> = std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.is_empty())
            .map(String::from)
            .collect();
        Self {
            cursor: entries.len(),
            entries,
            stash: String::new(),
        }
    }

    pub fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, self.entries.join("\n"));
    }

    fn push(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            self.cursor = self.entries.len();
            return;
        }
        if self.entries.last().map(String::as_str) != Some(line) {
            self.entries.push(line.to_owned());
        }
        self.cursor = self.entries.len();
    }

    fn prev(&mut self, current: &str) -> Option<&str> {
        if self.cursor == self.entries.len() {
            self.stash = current.to_owned();
        }
        if self.cursor == 0 {
            return None;
        }
        self.cursor -= 1;
        Some(&self.entries[self.cursor])
    }

    fn next(&mut self) -> Option<&str> {
        if self.cursor >= self.entries.len() {
            return None;
        }
        self.cursor += 1;
        match self.cursor == self.entries.len() {
            true => Some(&self.stash),
            false => Some(&self.entries[self.cursor]),
        }
    }
}

/// The text being typed, as lines and a cursor into them.
struct Buffer {
    lines: Vec<String>,
    /// Row, then column in characters — never bytes, so a multi-byte
    /// character moves the cursor once.
    cursor: (usize, usize),
}

impl Buffer {
    fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: (0, 0),
        }
    }

    fn from_str(text: &str) -> Self {
        let lines: Vec<String> = match text.is_empty() {
            true => vec![String::new()],
            false => text.lines().map(String::from).collect(),
        };
        let last = lines.len() - 1;
        let col = lines[last].chars().count();
        Self {
            cursor: (last, col),
            lines,
        }
    }

    fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    fn content(&self) -> String {
        self.lines.join("\n")
    }

    fn newline(&mut self) {
        let (row, col) = self.cursor;
        let at = byte_of(&self.lines[row], col);
        let rest = self.lines[row][at..].to_owned();
        self.lines[row].truncate(at);
        self.lines.insert(row + 1, rest);
        self.cursor = (row + 1, 0);
    }

    fn key(&mut self, code: KeyCode) {
        let (row, col) = self.cursor;
        match code {
            KeyCode::Backspace if col == 0 && row > 0 => {
                let joined = self.lines.remove(row);
                self.cursor = (row - 1, self.lines[row - 1].chars().count());
                self.lines[row - 1].push_str(&joined);
            }
            KeyCode::Left if col == 0 && row > 0 => {
                self.cursor = (row - 1, self.lines[row - 1].chars().count());
            }
            KeyCode::Left => self.cursor.1 = col.saturating_sub(1),
            KeyCode::Right if col >= self.lines[row].chars().count() => {
                if row + 1 < self.lines.len() {
                    self.cursor = (row + 1, 0);
                }
            }
            KeyCode::Right => self.cursor.1 += 1,
            KeyCode::Home => self.cursor.1 = 0,
            KeyCode::End => self.cursor.1 = self.lines[row].chars().count(),
            KeyCode::Backspace => {
                let (from, to) = (
                    byte_of(&self.lines[row], col - 1),
                    byte_of(&self.lines[row], col),
                );
                self.lines[row].drain(from..to);
                self.cursor.1 -= 1;
            }
            KeyCode::Delete if col < self.lines[row].chars().count() => {
                let (from, to) = (
                    byte_of(&self.lines[row], col),
                    byte_of(&self.lines[row], col + 1),
                );
                self.lines[row].drain(from..to);
            }
            KeyCode::Char(ch) => {
                let at = byte_of(&self.lines[row], col);
                self.lines[row].insert(at, ch);
                self.cursor.1 += 1;
            }
            _ => {}
        }
    }

    fn up(&mut self) {
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            self.cursor.1 = self.cursor.1.min(self.lines[self.cursor.0].chars().count());
        }
    }

    fn down(&mut self) {
        if self.cursor.0 + 1 < self.lines.len() {
            self.cursor.0 += 1;
            self.cursor.1 = self.cursor.1.min(self.lines[self.cursor.0].chars().count());
        }
    }
}

pub struct Input {
    buf: Buffer,
    pub history: History,
}

impl Input {
    pub fn new(history: History) -> Self {
        Self {
            buf: Buffer::new(),
            history,
        }
    }

    pub fn clear(&mut self) {
        self.buf = Buffer::new();
    }

    /// Lines of text, plus the box's own two border rows.
    pub fn height(&self) -> u16 {
        self.buf.lines.len() as u16 + 2
    }

    pub fn key(&mut self, key: KeyEvent) -> Action {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('c') if ctrl => return Action::Interrupt,
            KeyCode::Char('d') if ctrl && self.buf.is_empty() => return Action::Eof,
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => self.buf.newline(),
            KeyCode::Enter => {
                let content = self.buf.content();
                self.history.push(&content);
                self.buf = Buffer::new();
                return Action::Submit(content);
            }
            KeyCode::Up if self.buf.lines.len() > 1 && self.buf.cursor.0 > 0 => self.buf.up(),
            KeyCode::Up => {
                if let Some(entry) = self.history.prev(&self.buf.content()) {
                    self.buf = Buffer::from_str(entry);
                }
            }
            KeyCode::Down if self.buf.cursor.0 + 1 < self.buf.lines.len() => self.buf.down(),
            KeyCode::Down => {
                if let Some(entry) = self.history.next() {
                    self.buf = Buffer::from_str(entry);
                }
            }
            code => self.buf.key(code),
        }
        Action::Noop
    }

    /// Draw the box, and put the terminal cursor where the text cursor is.
    pub fn render(&self, frame: &mut Frame, area: Rect, agent: &str) {
        let title = Span::styled(
            format!(" {agent} "),
            Style::new().add_modifier(Modifier::BOLD | Modifier::DIM),
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().add_modifier(Modifier::DIM))
            .title(title);
        let inner = block.inner(area);
        frame.render_widget(Paragraph::new(self.lines()).block(block), area);

        let (row, col) = self.buf.cursor;
        frame.set_cursor_position((inner.x + col as u16 + 2, inner.y + row as u16));
    }

    fn lines(&self) -> Vec<Line<'static>> {
        self.buf
            .lines
            .iter()
            .enumerate()
            .map(|(at, line)| {
                let prompt = match at {
                    0 => "> ",
                    _ => "  ",
                };
                Line::from(vec![
                    Span::styled(prompt, Style::new().add_modifier(Modifier::DIM)),
                    Span::raw(line.clone()),
                ])
            })
            .collect()
    }
}

/// Byte offset of a character index, or the end of the string.
fn byte_of(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map(|(at, _)| at)
        .unwrap_or(text.len())
}
