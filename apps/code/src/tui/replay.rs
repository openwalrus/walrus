//! A stored session, back into the transcript.
//!
//! Resuming without this leaves the model holding a conversation the
//! person cannot see. What goes to scrollback is built from the same
//! items a live turn produces, so a replayed transcript and a streamed
//! one are the same thing rendered twice.

use crate::tui::item::{self, Item, ToolStatus};
use crabllm_core::Role;
use store::HistoryEntry;

/// Rows of history replayed at most.
///
/// This is rebuilding the terminal's own scrollback rather than an
/// internal buffer, so replaying more rows than a terminal keeps is work
/// nobody can scroll back to. Codex caps this per terminal and falls back
/// to a thousand rows for one it cannot identify; that fallback is what
/// this is.
pub const MAX_ROWS: usize = 1000;

/// The transcript a stored history reads as, oldest first.
pub fn items(history: &[HistoryEntry]) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    for entry in history {
        // A tool result arrives as a user message, so it is checked before
        // the role is.
        if !entry.tool_call_id().is_empty() {
            let output = entry.text().to_owned();
            let failed = item::failed(&output);
            if failed {
                settle(&mut out, ToolStatus::Failure);
            }
            out.push(Item::Result { output, failed });
            out.push(Item::Blank);
            continue;
        }
        match entry.role() {
            Role::System => {}
            Role::User if entry.text().is_empty() => {}
            Role::User => {
                out.push(Item::Prompt {
                    text: entry.text().to_owned(),
                });
                out.push(Item::Blank);
            }
            _ => {
                if !entry.text().is_empty() {
                    out.push(Item::Text {
                        md: entry.text().to_owned(),
                        marker: true,
                    });
                }
                let labels: Vec<String> = entry
                    .tool_calls()
                    .iter()
                    .map(|call| item::label(&call.function.name, &call.function.arguments, 80))
                    .collect();
                if !labels.is_empty() {
                    out.push(Item::Blank);
                    out.push(Item::Tool {
                        labels,
                        status: ToolStatus::Success,
                    });
                }
            }
        }
    }
    out
}

/// Mark the tool a result belongs to, which is the most recent one.
fn settle(items: &mut [Item], status: ToolStatus) {
    if let Some(Item::Tool { status: held, .. }) = items
        .iter_mut()
        .rev()
        .find(|item| matches!(item, Item::Tool { .. }))
    {
        *held = status;
    }
}
