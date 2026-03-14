//! Context compaction — summarize conversation history and replace it.

use crate::model::{Message, Model, Request};

const COMPACT_PROMPT: &str = include_str!("../../prompts/compact.md");

impl<M: Model> super::Agent<M> {
    /// Summarize the conversation history using the LLM.
    ///
    /// Sends the history with the compact prompt as system message.
    /// Returns the summary text, or `None` if the model produces no content.
    pub(crate) async fn compact(&self, history: &[Message]) -> Option<String> {
        let model_name = self
            .config
            .model
            .clone()
            .unwrap_or_else(|| self.model.active_model());

        let mut messages = Vec::with_capacity(2 + history.len());
        messages.push(Message::system(COMPACT_PROMPT));
        // Include the agent's system prompt as identity context so the
        // compaction LLM preserves <self>, <identity>, and <profile> info.
        if !self.config.system_prompt.is_empty() {
            messages.push(Message::user(format!(
                "Agent system prompt (preserve identity/profile info):\n{}",
                self.config.system_prompt
            )));
        }
        messages.extend(history.iter().cloned());

        let request = Request::new(model_name).with_messages(messages);
        match self.model.send(&request).await {
            Ok(response) => response.content().cloned(),
            Err(e) => {
                tracing::warn!("compaction LLM call failed: {e}");
                None
            }
        }
    }
}
