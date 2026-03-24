//! Tool schema and dispatch for the built-in `set_title` tool.

use crate::hook::DaemonHook;
use serde::Deserialize;
use wcore::{
    agent::{AsTool, ToolDescription},
    model::Tool,
};

/// Set a concise title for the current conversation.
#[derive(Deserialize, schemars::JsonSchema)]
pub(crate) struct SetTitle {
    /// A concise 3-6 word title summarizing the conversation topic.
    pub title: String,
}

impl ToolDescription for SetTitle {
    const DESCRIPTION: &'static str = "Set a concise title (3-6 words) for the current conversation. \
         Call this once on the first message of a new conversation to name it. \
         The title appears in the session list and filenames.";
}

pub(crate) fn tools() -> Vec<Tool> {
    vec![SetTitle::as_tool()]
}

impl DaemonHook {
    pub(crate) async fn dispatch_set_title(&self, args: &str, session_id: Option<u64>) -> String {
        let input: SetTitle = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };

        let Some(session_id) = session_id else {
            return "set_title requires a session".to_owned();
        };

        let title = input.title.trim().to_string();
        if title.is_empty() {
            return "title cannot be empty".to_owned();
        }

        self.pending_titles
            .lock()
            .await
            .insert(session_id, title.clone());
        format!("title set: {title}")
    }
}
