use crate::{
    cmd::auth::{AuthState, Focus, GATEWAY_NAMES, Tab},
    tui::{border_dim, border_focused, char_to_byte, handle_text_input, mask_token},
};
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

// ── Gateways key handling ───────────────────────────────────────────

pub(crate) fn handle_gateways_key(
    key: crossterm::event::KeyEvent,
    state: &mut AuthState,
) -> Result<Option<Result<()>>> {
    match state.focus {
        Focus::List => {
            match key.code {
                KeyCode::Char('q') => return Ok(Some(Ok(()))),
                KeyCode::Up | KeyCode::Char('k') => {
                    state.gateway_selected = state.gateway_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.gateway_selected < GATEWAY_NAMES.len() - 1 {
                        state.gateway_selected += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                    state.focus = Focus::Editing;
                    let token = &state.gateway_tokens[state.gateway_selected];
                    state.cursor = token.chars().count();
                    state.edit_buf = token.clone();
                }
                KeyCode::Char('x') | KeyCode::Delete => {
                    state.gateway_tokens[state.gateway_selected].clear();
                }
                _ => {}
            }
            Ok(None)
        }
        Focus::Editing => {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    state.gateway_tokens[state.gateway_selected] = state.edit_buf.clone();
                    state.focus = Focus::List;
                }
                _ => handle_text_input(key.code, &mut state.edit_buf, &mut state.cursor),
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

// ── Gateways rendering ──────────────────────────────────────────────

pub(crate) fn render_gateways(frame: &mut Frame, state: &AuthState, area: Rect) {
    let horiz =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).split(area);
    render_gateway_list(frame, state, horiz[0]);
    render_gateway_detail(frame, state, horiz[1]);
}

fn render_gateway_list(frame: &mut Frame, state: &AuthState, area: Rect) {
    let focused = state.tab == Tab::Gateways && state.focus == Focus::List;
    let block = Block::default()
        .title(" Gateways ")
        .borders(Borders::ALL)
        .border_style(if focused {
            border_focused()
        } else {
            border_dim()
        });

    let lines: Vec<Line> = GATEWAY_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let marker = if i == state.gateway_selected {
                "> "
            } else {
                "  "
            };
            let configured = if state.gateway_tokens[i].is_empty() {
                ""
            } else {
                " *"
            };
            let text = format!("{marker}{name}{configured}");
            let style = if i == state.gateway_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(text, style))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_gateway_detail(frame: &mut Frame, state: &AuthState, area: Rect) {
    let name = GATEWAY_NAMES[state.gateway_selected];
    let token = &state.gateway_tokens[state.gateway_selected];
    let editing = state.tab == Tab::Gateways && state.focus == Focus::Editing;

    let hints = [
        "https://core.telegram.org/bots#botfather",
        "https://discord.com/developers/applications",
    ];
    let hint = hints[state.gateway_selected];

    let block = Block::default()
        .title(format!(" {name} "))
        .borders(Borders::ALL)
        .border_style(if editing {
            border_focused()
        } else {
            border_dim()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let label_span = Span::styled(
        "     Token: ",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let line = if editing {
        let byte_pos = char_to_byte(&state.edit_buf, state.cursor);
        let mut s = state.edit_buf.clone();
        s.insert(byte_pos, '|');
        Line::from(vec![
            label_span,
            Span::styled(s, Style::default().fg(Color::Green)),
        ])
    } else if token.is_empty() {
        Line::from(vec![
            label_span,
            Span::styled(
                hint,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])
    } else {
        Line::from(vec![
            label_span,
            Span::styled(mask_token(token), Style::default().fg(Color::White)),
        ])
    };

    frame.render_widget(Paragraph::new(line), inner);
}
