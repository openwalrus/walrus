//! Markdown to ratatui lines, through termimad's wrapper and skin.
//!
//! termimad's own styles are not used. It re-exports a crossterm newer
//! than the one ratatui pins, so a `ContentStyle` cannot cross between
//! them; the skin is defined here, so what each kind looks like is known
//! without asking.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::sync::LazyLock;
use termimad::{CompositeKind, FmtLine, FmtText, MadSkin, minimad::Compound};

/// Left margin of body text, aligning it under the `⏺ ` marker.
pub const PAD: &str = "  ";

pub const BRAND: Color = Color::Indexed(173);
pub const GREEN: Color = Color::Indexed(71);
pub const RED: Color = Color::Indexed(204);
pub const SUBTLE: Color = Color::Indexed(240);

pub const S_DIM: Style = Style::new().add_modifier(Modifier::DIM);
pub const S_SUBTLE: Style = Style::new().fg(SUBTLE);

pub static SKIN: LazyLock<MadSkin> = LazyLock::new(|| {
    use termimad::crossterm::style::{Attribute, Color};

    let mut skin = MadSkin::default_dark();
    skin.paragraph.left_margin = 2;
    for (at, color) in [(0, Color::Cyan), (1, Color::Magenta), (2, Color::White)] {
        skin.headers[at]
            .compound_style
            .set_fgbg(color, Color::Reset);
        skin.headers[at].compound_style.add_attr(Attribute::Bold);
        skin.headers[at].left_margin = 2;
    }
    skin.code_block.left_margin = 4;
    skin
});

/// Render `md` at `width`, wrapped and styled.
pub fn lines(md: &str, width: usize) -> Vec<Line<'static>> {
    let fmt = FmtText::from(&SKIN, md, Some(width));
    let mut out = Vec::with_capacity(fmt.lines.len());
    for line in &fmt.lines {
        match line {
            FmtLine::Normal(composite) => {
                let base = kind_style(composite.kind);
                let mut spans = vec![Span::raw(" ".repeat(kind_margin(composite.kind)))];
                for compound in &composite.compounds {
                    let extra = modifiers(compound);
                    let style = match extra.is_empty() {
                        true => base,
                        false => base.add_modifier(extra),
                    };
                    spans.push(Span::styled(compound.src.to_string(), style));
                }
                out.push(Line::from(spans));
            }
            FmtLine::TableRow(row) => {
                let mut spans = vec![Span::raw("  │")];
                for cell in &row.cells {
                    for compound in &cell.compounds {
                        spans.push(Span::raw(compound.src.to_string()));
                    }
                    spans.push(Span::raw("│"));
                }
                out.push(Line::from(spans));
            }
            FmtLine::TableRule(rule) => {
                let total = rule.widths.iter().sum::<usize>() + rule.widths.len() + 1;
                out.push(Line::raw(format!("  {}", "─".repeat(total))));
            }
            FmtLine::HorizontalRule => out.push(Line::raw(format!(
                "  {}",
                "─".repeat(width.saturating_sub(2))
            ))),
        }
    }
    out
}

/// Drop a line's leading spaces so a marker can take their place.
pub fn unindent(line: Line<'static>) -> Vec<Span<'static>> {
    line.spans
        .into_iter()
        .skip_while(|span| span.content.chars().all(|c| c == ' '))
        .collect()
}

/// Mirrors [`SKIN`] — the header colours it sets, read back.
fn kind_style(kind: CompositeKind) -> Style {
    let color = match kind {
        CompositeKind::Header(1) => Color::Cyan,
        CompositeKind::Header(2) => Color::Magenta,
        CompositeKind::Header(_) => Color::White,
        _ => return Style::default(),
    };
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

fn kind_margin(kind: CompositeKind) -> usize {
    match kind {
        CompositeKind::Code => 4,
        _ => PAD.len(),
    }
}

fn modifiers(compound: &Compound<'_>) -> Modifier {
    let mut out = Modifier::empty();
    if compound.bold {
        out |= Modifier::BOLD;
    }
    if compound.italic {
        out |= Modifier::ITALIC;
    }
    if compound.strikeout {
        out |= Modifier::CROSSED_OUT;
    }
    out
}
