//! The transcript as it arrives: chunks in, items out.
//!
//! A line is the unit. Text is held until its newline, a fence until it
//! closes, a tool until it settles — and only then does it become an
//! item, because an item is what goes to scrollback and scrollback
//! cannot be edited afterwards.

use crate::tui::{
    item::{self, Item, ToolStatus},
    markdown::{PAD, S_DIM},
};
use ratatui::text::{Line, Span};

#[derive(Default)]
enum State {
    #[default]
    Normal,
    Code {
        code: String,
    },
    Table {
        rows: String,
    },
    Thinking,
}

pub struct Transcript {
    pub items: Vec<Item>,
    /// True while a turn is in flight with nothing shown for it yet.
    pub waiting: bool,
    /// How many items have reached scrollback.
    committed: usize,
    /// The line being typed into by the model.
    line: String,
    state: State,
    /// Whether the next rendered line opens a response and takes the `⏺`.
    marker: bool,
    started: bool,
    after_tool: bool,
    thinking: String,
    tool_failed: bool,
    width: usize,
}

impl Transcript {
    pub fn new(width: usize) -> Self {
        Self {
            items: Vec::new(),
            waiting: false,
            committed: 0,
            line: String::new(),
            state: State::default(),
            marker: false,
            started: false,
            after_tool: false,
            thinking: String::new(),
            tool_failed: false,
            width,
        }
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    /// Items that became final since the last call, in order.
    pub fn settled(&mut self) -> &[Item] {
        let from = self.committed;
        while self
            .items
            .get(self.committed)
            .is_some_and(|item| item.settled())
        {
            self.committed += 1;
        }
        &self.items[from..self.committed]
    }

    /// Items still in flight, which the viewport draws until they settle.
    pub fn live(&self) -> &[Item] {
        &self.items[self.committed..]
    }

    /// The partial line, drawn under the live items.
    pub fn current(&self) -> Option<Line<'static>> {
        if self.line.is_empty() {
            return None;
        }
        let prefix = match self.marker || !self.started {
            true => Span::styled("⏺ ", S_DIM),
            false => Span::raw(PAD),
        };
        Some(Line::from(vec![prefix, Span::raw(self.line.clone())]))
    }

    /// A turn has been sent and nothing has come back yet.
    pub fn start(&mut self) {
        self.waiting = true;
        self.started = false;
        self.marker = false;
        self.after_tool = false;
        self.tool_failed = false;
    }

    pub fn push_text(&mut self, chunk: &str) {
        if chunk.is_empty() {
            return;
        }
        self.waiting = false;
        if matches!(self.state, State::Thinking) {
            self.flush_thinking();
        }
        if !self.started {
            self.started = true;
            self.marker = true;
        }
        if self.after_tool {
            self.after_tool = false;
            self.marker = true;
        }

        for ch in chunk.chars() {
            if ch != '\n' {
                self.line.push(ch);
                continue;
            }
            let line = std::mem::take(&mut self.line);
            let had_content = !line.is_empty();
            self.line(&line);
            // A blank line at the start of a response must not eat the
            // marker the first real one is owed.
            if had_content {
                self.marker = false;
            }
        }
    }

    pub fn push_thinking(&mut self, chunk: &str) {
        if chunk.is_empty() {
            return;
        }
        self.waiting = false;
        self.state = State::Thinking;
        self.thinking.push_str(chunk);
    }

    pub fn push_tool_start(&mut self, calls: &[(String, String)]) {
        self.flush_thinking();
        self.flush_line();
        self.waiting = false;
        self.tool_failed = false;
        self.items.push(Item::Blank);
        self.items.push(Item::Tool {
            labels: calls
                .iter()
                .map(|(name, args)| item::label(name, args, self.width))
                .collect(),
            status: ToolStatus::Running,
        });
    }

    pub fn push_tool_result(&mut self, output: &str) {
        let failed = item::failed(output);
        self.tool_failed |= failed;
        self.items.push(Item::Result {
            output: output.to_owned(),
            failed,
        });
        self.items.push(Item::Blank);
    }

    /// Settle the running tool, which is what lets everything queued
    /// behind it reach scrollback.
    pub fn push_tool_done(&mut self) {
        let status = match self.tool_failed {
            true => ToolStatus::Failure,
            false => ToolStatus::Success,
        };
        for item in self.items.iter_mut().rev() {
            if let Item::Tool { status: held, .. } = item
                && *held == ToolStatus::Running
            {
                *held = status;
                break;
            }
        }
        self.after_tool = true;
    }

    /// End of turn: nothing more is coming, so nothing stays in flight.
    pub fn finish(&mut self) {
        self.push_tool_done();
        self.flush_thinking();
        self.flush_line();
        self.waiting = false;
        self.after_tool = false;
    }

    /// Push a line the caller wrote rather than the model — a prompt
    /// echo, an error, a notice.
    pub fn push(&mut self, item: Item) {
        self.items.push(item);
    }

    // ── Internal ────────────────────────────────────────────────

    fn line(&mut self, line: &str) {
        match &mut self.state {
            State::Code { code } => match line.starts_with("```") {
                true => {
                    let code = std::mem::take(code);
                    self.items.push(Item::Code { code });
                    self.state = State::Normal;
                }
                false => {
                    code.push_str(line);
                    code.push('\n');
                }
            },
            State::Table { rows } => match line.starts_with('|') {
                true => {
                    rows.push_str(line);
                    rows.push('\n');
                }
                false => {
                    self.flush_table();
                    self.line(line);
                }
            },
            State::Normal | State::Thinking => {
                if let Some(rest) = line.strip_prefix("```") {
                    self.open_code(rest.trim());
                } else if line.starts_with('|') {
                    self.state = State::Table {
                        rows: format!("{line}\n"),
                    };
                } else if line.is_empty() {
                    self.items.push(Item::Blank);
                } else {
                    self.items.push(Item::Text {
                        md: line.to_owned(),
                        marker: self.marker,
                    });
                }
            }
        }
    }

    /// The opening border is its own item: it is known the moment the
    /// fence opens, where the block itself is not known until it closes.
    fn open_code(&mut self, lang: &str) {
        let label = match lang.is_empty() {
            true => "┌─".to_owned(),
            false => format!("┌ {lang} ─"),
        };
        self.items.push(Item::Border { label });
        self.marker = false;
        self.state = State::Code {
            code: String::new(),
        };
    }

    fn flush_table(&mut self) {
        if let State::Table { rows } = &mut self.state {
            let rows = std::mem::take(rows);
            self.items.push(Item::Table { rows });
        }
        self.state = State::Normal;
    }

    /// Whatever is half-written becomes an item, because the thing that
    /// would have completed it is not coming.
    fn flush_line(&mut self) {
        if self.line.is_empty() {
            return;
        }
        let line = std::mem::take(&mut self.line);
        match &mut self.state {
            State::Code { code } => {
                let code = format!("{code}{line}");
                self.items.push(Item::Code { code });
                self.state = State::Normal;
            }
            _ => self.items.push(Item::Text {
                md: line,
                marker: self.marker,
            }),
        }
        self.marker = false;
    }

    fn flush_thinking(&mut self) {
        if !self.thinking.is_empty() {
            let text = std::mem::take(&mut self.thinking);
            self.items.push(Item::Thinking { text });
        }
        if matches!(self.state, State::Thinking) {
            self.state = State::Normal;
        }
    }
}
