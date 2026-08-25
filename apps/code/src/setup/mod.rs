//! First run: naming an endpoint, before there is one to talk to.
//!
//! It writes `[llm]` into the shared config, so what is set here is what
//! the daemon reads too — there is one endpoint per install, not one per
//! product.

use anyhow::Result;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use ratatui::{
    Frame, Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

mod login;

/// Where a terminal logs in.
const CLOUD: &str = "https://api-beta.crabtalk.ai";

/// The endpoints an API key can name. `Custom` is not offered: it needs a
/// base URL as well, which is a config file rather than a prompt.
const KINDS: &[&str] = &["anthropic", "openai", "google", "ollama", "azure"];

/// Rows the flow draws into — its longest screen, since an inline
/// viewport cannot be resized after it is built.
const VIEWPORT: u16 = 10;

/// Whether nothing names an endpoint yet.
pub fn needed(config: &store::Config) -> bool {
    !config.llm.is_set() && config.providers.is_empty()
}

/// Ask how to reach a model, and write the answer to the config.
///
/// `Ok(false)` if the person walked away without choosing.
pub async fn run() -> Result<bool> {
    enable_raw_mode()?;
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        hook(info);
    }));

    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(std::io::stdout()),
        TerminalOptions {
            viewport: Viewport::Inline(VIEWPORT),
        },
    )?;
    let result = ask(&mut terminal).await;

    disable_raw_mode()?;
    terminal.clear()?;
    result
}

/// Which screen the flow is on.
enum Screen {
    /// Log in, or name a provider.
    Method {
        at: usize,
    },
    Provider {
        at: usize,
    },
    Key {
        kind: String,
        key: String,
    },
    /// The browser is open and the callback has not arrived.
    Waiting,
}

async fn ask(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<bool> {
    let mut screen = Screen::Method { at: 0 };
    let mut keys = EventStream::new();

    loop {
        terminal.draw(|frame| view(frame, &screen))?;

        // The browser flow waits on a socket rather than on a keystroke.
        // Both are awaited so the wait can still be walked away from, and
        // the future is held across the loop so a stray key does not
        // reopen the browser.
        if matches!(screen, Screen::Waiting) {
            let mut pending = std::pin::pin!(login::token(CLOUD));
            loop {
                tokio::select! {
                    token = &mut pending => {
                        write(&[("base_url", format!("{CLOUD}/v1")), ("api_key", token?)])?;
                        return Ok(true);
                    }
                    event = keys.next() => {
                        let Some(Ok(Event::Key(key))) = event else { continue };
                        if key.code == KeyCode::Esc {
                            return Ok(false);
                        }
                    }
                }
            }
        }

        let Some(event) = keys.next().await else {
            return Ok(false);
        };
        let Event::Key(key) = event? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match (&mut screen, key.code) {
            (_, KeyCode::Esc) => return Ok(false),
            (Screen::Method { at }, KeyCode::Up) => *at = at.saturating_sub(1),
            (Screen::Method { at }, KeyCode::Down) => *at = (*at + 1).min(1),
            (Screen::Method { at }, KeyCode::Enter) => {
                screen = match at {
                    0 => Screen::Waiting,
                    _ => Screen::Provider { at: 0 },
                };
            }
            (Screen::Provider { at }, KeyCode::Up) => *at = at.saturating_sub(1),
            (Screen::Provider { at }, KeyCode::Down) => *at = (*at + 1).min(KINDS.len() - 1),
            (Screen::Provider { at }, KeyCode::Enter) => {
                screen = Screen::Key {
                    kind: KINDS[*at].to_owned(),
                    key: String::new(),
                };
            }
            (Screen::Key { key, .. }, KeyCode::Char(ch)) => key.push(ch),
            (Screen::Key { key, .. }, KeyCode::Backspace) => {
                key.pop();
            }
            (Screen::Key { kind, key }, KeyCode::Enter) if !key.is_empty() => {
                write(&[("kind", kind.clone()), ("api_key", key.clone())])?;
                return Ok(true);
            }
            _ => {}
        }
    }
}

fn view(frame: &mut Frame, screen: &Screen) {
    let dim = Style::new().add_modifier(Modifier::DIM);
    let [title, body, hint] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "  crab needs a model to talk to",
                Style::new().add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
        ]),
        title,
    );

    let lines = match screen {
        Screen::Method { at } => options(&["Log in with crabtalk", "Use an API key"], *at),
        Screen::Provider { at } => options(KINDS, *at),
        Screen::Key { kind, key } => vec![
            Line::from(Span::styled(format!("  {kind} api key"), dim)),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  ", dim),
                Span::raw("•".repeat(key.chars().count())),
                Span::styled("▏", dim),
            ]),
        ],
        Screen::Waiting => vec![Line::from(Span::styled("  waiting for the browser…", dim))],
    };
    frame.render_widget(Paragraph::new(lines), body);

    let keys = match screen {
        Screen::Key { .. } => "  enter to save  ·  esc to cancel",
        Screen::Waiting => "  esc to cancel",
        _ => "  ↑↓ to choose  ·  enter to select  ·  esc to cancel",
    };
    frame.render_widget(Paragraph::new(Line::from(Span::styled(keys, dim))), hint);
}

fn options(labels: &[&str], at: usize) -> Vec<Line<'static>> {
    labels
        .iter()
        .enumerate()
        .map(|(idx, label)| {
            let selected = idx == at;
            let marker = match selected {
                true => "  ❯ ",
                false => "    ",
            };
            let style = match selected {
                true => Style::new().add_modifier(Modifier::BOLD),
                false => Style::new().add_modifier(Modifier::DIM),
            };
            Line::from(vec![
                Span::styled(marker, style),
                Span::styled((*label).to_owned(), style),
            ])
        })
        .collect()
}

/// Merge keys into `[llm]`, leaving the rest of the file as written.
fn write(entries: &[(&str, String)]) -> Result<()> {
    let path = &*crabup::dirs::CONFIG_FILE;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut doc: toml::Table = toml::from_str(&raw).unwrap_or_default();

    let llm = doc
        .entry("llm")
        .or_insert_with(|| toml::Value::Table(Default::default()));
    if let Some(table) = llm.as_table_mut() {
        for (key, value) in entries {
            table.insert((*key).to_owned(), toml::Value::String(value.clone()));
        }
    }
    Ok(std::fs::write(path, toml::to_string_pretty(&doc)?)?)
}
