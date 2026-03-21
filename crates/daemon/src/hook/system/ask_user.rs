//! Tool schema and dispatch for the built-in `ask_user` tool.

use crate::hook::DaemonHook;
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::oneshot;
use wcore::{
    agent::{AsTool, ToolDescription},
    model::Tool,
};

/// Ask the user one or more questions and wait for their reply.
#[derive(Deserialize, schemars::JsonSchema)]
pub(crate) struct AskUser {
    /// The questions to ask the user.
    pub questions: Vec<String>,
}

impl ToolDescription for AskUser {
    const DESCRIPTION: &'static str = "Ask the user one or more questions and wait for their reply. Use when you need clarification or approval before proceeding.";
}

pub(crate) fn tools() -> Vec<Tool> {
    vec![AskUser::as_tool()]
}

/// Timeout for waiting on user reply (5 minutes).
const ASK_USER_TIMEOUT: Duration = Duration::from_secs(300);

impl DaemonHook {
    pub(crate) async fn dispatch_ask_user(&self, args: &str, session_id: Option<u64>) -> String {
        let input: AskUser = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };

        let session_id = match session_id {
            Some(id) => id,
            None => return "ask_user is only available in streaming mode".to_owned(),
        };

        let (tx, rx) = oneshot::channel();
        self.pending_asks.lock().await.insert(session_id, tx);

        match tokio::time::timeout(ASK_USER_TIMEOUT, rx).await {
            Ok(Ok(reply)) => reply,
            Ok(Err(_)) => {
                self.pending_asks.lock().await.remove(&session_id);
                "ask_user cancelled: reply channel closed".to_owned()
            }
            Err(_) => {
                self.pending_asks.lock().await.remove(&session_id);
                format!(
                    "ask_user timed out after {}s: no reply received for: {}",
                    ASK_USER_TIMEOUT.as_secs(),
                    input.questions.join("; "),
                )
            }
        }
    }
}
