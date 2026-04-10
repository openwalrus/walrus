//! Ask-user tool handler factory.

use runtime::host::Host;
use serde::Deserialize;
use std::sync::Arc;
use wcore::{
    ToolDispatch, ToolHandler,
    agent::{AsTool, ToolDescription},
    model::Tool,
};

/// A single option the user can choose from.
#[derive(Deserialize, schemars::JsonSchema)]
pub struct QuestionOption {
    /// Concise option label (1-5 words).
    pub label: String,
    /// Explanation of the choice.
    pub description: String,
}

/// A structured question with predefined options.
#[derive(Deserialize, schemars::JsonSchema)]
pub struct Question {
    /// Full question text.
    pub question: String,
    /// Short UI title for the question (max 12 chars, e.g. "Database").
    pub header: String,
    /// Predefined choices for the user.
    pub options: Vec<QuestionOption>,
    /// Allow multiple selections.
    #[serde(default)]
    pub multi_select: bool,
}

/// Ask the user one or more structured questions and wait for their reply.
#[derive(Deserialize, schemars::JsonSchema)]
pub struct AskUser {
    /// The questions to ask the user.
    pub questions: Vec<Question>,
}

impl ToolDescription for AskUser {
    const DESCRIPTION: &'static str = r#"Ask the user one or more structured questions with predefined options. Each question needs a short UI header, the full question text, and options with labels and descriptions. The user picks from the options or types a free-text "Other" answer. Returns JSON mapping question text to selected label. For multi_select, the answer is a comma-joined string like "Option A, Option B"."#;
}

pub fn handler<H: Host + 'static>(host: H) -> (Tool, ToolHandler) {
    (
        AskUser::as_tool(),
        Arc::new(move |call: ToolDispatch| {
            let host = host.clone();
            Box::pin(async move {
                host.dispatch_ask_user(&call.args, call.conversation_id)
                    .await
            })
        }),
    )
}
