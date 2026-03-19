use crate::{
    cmd::auth::{AuthState, Focus, Tab},
    tui::{border_dim, border_focused, char_to_byte, handle_text_input, mask_token},
};
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

// ── Gateways key handling ───────────────────────────────────────────

pub(crate) fn handle_gateways_key(
    key: crossterm::event::KeyEvent,
    state: &mut AuthState,
) -> Result<Option<Result<()>>> {
    if state.gateways.is_empty() {
        return match key.code {
            KeyCode::Char('q') => Ok(Some(Ok(()))),
            _ => Ok(None),
        };
    }

    match state.focus {
        Focus::List => {
            match key.code {
                KeyCode::Char('q') => return Ok(Some(Ok(()))),
                KeyCode::Up | KeyCode::Char('k') => {
                    state.gateway_selected = state.gateway_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.gateway_selected + 1 < state.gateways.len() {
                        state.gateway_selected += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                    state.focus = Focus::Editing;
                    let token = &state.gateways[state.gateway_selected].token;
                    state.cursor = token.chars().count();
                    state.edit_buf = token.clone();
                }
                KeyCode::Char('x') | KeyCode::Delete => {
                    state.gateways[state.gateway_selected].token.clear();
                }
                _ => {}
            }
            Ok(None)
        }
        Focus::Editing => {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    state.gateways[state.gateway_selected].token = state.edit_buf.clone();
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
    if state.gateways.is_empty() {
        render_placeholder(frame, area);
        return;
    }
    let horiz =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).split(area);
    render_gateway_list(frame, state, horiz[0]);
    render_gateway_detail(frame, state, horiz[1]);
}

fn render_placeholder(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Gateways ")
        .borders(Borders::ALL)
        .border_style(border_dim());

    let text = Paragraph::new(vec![
        Line::raw(""),
        Line::styled(
            "No gateways configured.",
            Style::default().fg(Color::DarkGray),
        ),
        Line::raw(""),
        Line::styled(
            "Find one at https://crabtalk.ai/hub?type=gateway",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::ITALIC),
        ),
    ])
    .alignment(Alignment::Center)
    .block(block);

    frame.render_widget(text, area);
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

    let lines: Vec<Line> = state
        .gateways
        .iter()
        .enumerate()
        .map(|(i, gw)| {
            let marker = if i == state.gateway_selected {
                "> "
            } else {
                "  "
            };
            let configured = if gw.token.is_empty() { "" } else { " *" };
            let text = format!("{marker}{}{configured}", gw.name);
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
    let Some(gw) = state.gateways.get(state.gateway_selected) else {
        return;
    };
    let editing = state.tab == Tab::Gateways && state.focus == Focus::Editing;

    let block = Block::default()
        .title(format!(" {} ", gw.name))
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
    } else if gw.token.is_empty() {
        Line::from(vec![
            label_span,
            Span::styled(
                "(empty)",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])
    } else {
        Line::from(vec![
            label_span,
            Span::styled(mask_token(&gw.token), Style::default().fg(Color::White)),
        ])
    };

    frame.render_widget(Paragraph::new(line), inner);
}
