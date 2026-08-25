//! The transcript, as source rather than as lines.
//!
//! An item keeps what it was written from, and renders to lines at a
//! width asked for at the time. Keeping the rendering instead would make
//! a resize unanswerable: the terminal reflows its own scrollback, but
//! not lines something else already wrapped.

use crate::tui::markdown::{self, BRAND, GREEN, PAD, RED, S_DIM, S_SUBTLE, SUBTLE};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Lines of a tool's output shown before the rest is elided.
const RESULT_LINES_OK: usize = 5;
const RESULT_LINES_FAILED: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Success,
    Failure,
}

/// One logical chunk of the transcript.
#[derive(Debug, Clone)]
pub enum Item {
    /// A markdown line. `marker` gives it the `⏺` that opens a response.
    Text {
        md: String,
        marker: bool,
    },
    /// What was typed, echoed into the transcript so scrollback reads as
    /// a conversation rather than as one side of one.
    Prompt {
        text: String,
    },
    /// The line a fence opens with, drawn before the block it belongs to
    /// exists.
    Border {
        label: String,
    },
    /// A fenced block, kept whole: it is not a block until it closes.
    Code {
        code: String,
    },
    /// Rows of a table, which wrap as a unit rather than a line at a time.
    Table {
        rows: String,
    },
    /// One round of tool calls, by label.
    Tool {
        labels: Vec<String>,
        status: ToolStatus,
    },
    /// A tool's output, as it came back.
    Result {
        output: String,
        failed: bool,
    },
    Thinking {
        text: String,
    },
    Blank,
}

impl Item {
    /// Whether this is final. A running tool is not: its marker changes
    /// when it settles, and a line already in scrollback cannot.
    pub fn settled(&self) -> bool {
        !matches!(
            self,
            Item::Tool {
                status: ToolStatus::Running,
                ..
            }
        )
    }

    /// `frame` drives the spinner on a tool that is still running.
    pub fn render(&self, width: usize, frame: u64) -> Vec<Line<'static>> {
        match self {
            Item::Text { md, marker } => {
                let mut lines = markdown::lines(md, width);
                if *marker && !lines.is_empty() {
                    let first = markdown::unindent(lines.remove(0));
                    let mut spans = vec![Span::styled("⏺ ", S_DIM)];
                    spans.extend(first);
                    lines.insert(0, Line::from(spans));
                }
                lines
            }
            Item::Prompt { text } => text
                .lines()
                .map(|line| {
                    Line::from(vec![
                        Span::styled("> ", Style::new().fg(BRAND)),
                        Span::styled(line.to_owned(), S_DIM),
                    ])
                })
                .collect(),
            Item::Border { label } => vec![Line::from(vec![
                Span::raw(PAD),
                Span::styled(label.clone(), S_SUBTLE),
            ])],
            Item::Code { code } => {
                let mut lines: Vec<Line> = code
                    .lines()
                    .map(|line| {
                        Line::from(vec![
                            Span::raw(PAD),
                            Span::styled("│ ", Style::new().fg(SUBTLE)),
                            Span::raw(line.to_owned()),
                        ])
                    })
                    .collect();
                lines.push(Line::from(vec![
                    Span::raw(PAD),
                    Span::styled("└─", S_SUBTLE),
                ]));
                lines
            }
            Item::Table { rows } => markdown::lines(rows, width.saturating_sub(PAD.len())),
            Item::Tool { labels, status } => {
                let (marker, style) = match status {
                    ToolStatus::Running => (Span::styled(spinner(frame), S_DIM), S_DIM),
                    ToolStatus::Success => (
                        Span::styled("⏺ ", Style::new().fg(GREEN)),
                        Style::new().add_modifier(Modifier::BOLD | Modifier::DIM),
                    ),
                    ToolStatus::Failure => (
                        Span::styled("⏺ ", Style::new().fg(RED)),
                        Style::new().add_modifier(Modifier::BOLD | Modifier::DIM),
                    ),
                };
                labels
                    .iter()
                    .map(|label| {
                        Line::from(vec![marker.clone(), Span::styled(label.clone(), style)])
                    })
                    .collect()
            }
            Item::Result { output, failed } => result_lines(output, *failed, width),
            Item::Thinking { text } => text
                .lines()
                .map(|line| {
                    Line::from(Span::styled(
                        format!("{PAD}{line}"),
                        Style::new().add_modifier(Modifier::DIM | Modifier::ITALIC),
                    ))
                })
                .collect(),
            Item::Blank => vec![Line::raw("")],
        }
    }
}

/// Whether a tool's output reads as a failure.
pub fn failed(output: &str) -> bool {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output)
        && let Some(code) = value.get("exit_code").and_then(|c| c.as_i64())
    {
        return code != 0;
    }
    ["bash failed:", "tool not available:", "invalid arguments:"]
        .iter()
        .any(|prefix| output.starts_with(prefix))
}

/// A tool call named the way the transcript names it: the tool, and for
/// a shell the command itself, since `Bash` alone says nothing.
pub fn label(name: &str, args: &str, width: usize) -> String {
    use heck::ToUpperCamelCase;

    let pascal = name.to_upper_camel_case();
    if name != "bash" {
        return pascal;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(args) else {
        return pascal;
    };
    let Some(command) = value.get("command").and_then(|c| c.as_str()) else {
        return pascal;
    };
    let first = command.lines().next().unwrap_or(command);
    let max = width.saturating_sub(8);
    let shown = match (first.len() > max, first.len() < command.len()) {
        (true, _) => format!("{}...", &first[..max]),
        (false, true) => format!("{first}..."),
        _ => first.to_owned(),
    };
    format!("Bash({shown})")
}

fn result_lines(output: &str, failed: bool, width: usize) -> Vec<Line<'static>> {
    let max = match failed {
        true => RESULT_LINES_FAILED,
        false => RESULT_LINES_OK,
    };
    let (shown, total) = elide(output, max);
    let room = width.saturating_sub(PAD.len() + 2);

    let mut lines = Vec::new();
    if shown.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(PAD),
            Span::styled("⎿ ", S_SUBTLE),
            Span::styled("(no output)", S_DIM),
        ]));
        return lines;
    }
    for (at, line) in shown.iter().enumerate() {
        let text = match line.len() > room {
            true => format!("{}...", &line[..room.saturating_sub(3)]),
            false => line.clone(),
        };
        lines.push(match at {
            0 => Line::from(vec![
                Span::raw(PAD),
                Span::styled("⎿ ", S_SUBTLE),
                Span::styled(text, S_DIM),
            ]),
            _ => Line::from(vec![
                Span::raw(format!("{PAD}  ")),
                Span::styled(text, S_DIM),
            ]),
        });
    }
    if total > shown.len() {
        lines.push(Line::from(vec![
            Span::raw(format!("{PAD}  ")),
            Span::styled(format!("… +{} lines", total - shown.len()), S_DIM),
        ]));
    }
    lines
}

/// The first `max` non-empty lines, and how many there were. A shell
/// result arrives as JSON, and what is worth showing is the stream that
/// has something in it.
fn elide(output: &str, max: usize) -> (Vec<String>, usize) {
    let text = match serde_json::from_str::<serde_json::Value>(output) {
        Ok(value) => {
            let field = |name: &str| {
                value
                    .get(name)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned()
            };
            let (stdout, stderr) = (field("stdout"), field("stderr"));
            match value.get("exit_code").and_then(|c| c.as_i64()) {
                Some(0) | None if !stdout.is_empty() => stdout,
                Some(_) if !stderr.is_empty() => stderr,
                Some(_) | None => match stdout.is_empty() {
                    true => stderr,
                    false => stdout,
                },
            }
        }
        Err(_) => output.to_owned(),
    };
    let all: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();
    let total = all.len();
    (all.into_iter().take(max).map(String::from).collect(), total)
}

/// Braille spinner, trailing space included so it drops in where a
/// settled `⏺ ` marker would go.
fn spinner(frame: u64) -> String {
    const BRAILLE: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    format!("{} ", BRAILLE[(frame as usize / 2) % BRAILLE.len()])
}
