//! Llama3 prompt formatter

use crate::chat::{Formatter, Message};

/// Llama3 prompt formatter
#[derive(Default)]
pub struct Llama3 {
    output: String,
}

impl Llama3 {
    /// Emit the begin of text token
    fn emit_begin(&mut self) {
        self.output.push_str("<|begin_of_text|>");
    }

    /// Add a default system message
    fn default_system(&mut self) {
        self.output
            .push_str("<|start_header_id|>system<|end_header_id|>\n");
        self.output
            .push_str("You are a helpful assistant.<|eot_id|>");
    }

    fn emit_system(&mut self, system: &str) {
        self.output.push_str(&format!(
            "<|start_header_id|>system<|end_header_id|>\n\n{system}<|eot_id|>"
        ));
    }

    fn emit_user(&mut self, user: &str) {
        self.output.push_str(&format!(
            "<|start_header_id|>user<|end_header_id|>\n\n{user}<|eot_id|>"
        ));
    }

    fn emit_assistant(&mut self, assistant: &str) {
        self.output.push_str(&format!(
            "<|start_header_id|>assistant<|end_header_id|>\n\n{assistant}<|eot_id|>"
        ));
    }

    fn emit_complete(&mut self) {
        self.output
            .push_str("<|start_header_id|>assistant<|end_header_id|>\n\n");
    }
}

impl Formatter for Llama3 {
    const EOS_TOKEN: &str = "<|eot_id|>";

    fn format(messages: &[Message]) -> anyhow::Result<String> {
        let mut formatter = Llama3::default();
        formatter.emit_begin();
        formatter.default_system();

        for message in messages {
            match message {
                Message::System(system) => formatter.emit_system(system),
                Message::User(user) => formatter.emit_user(user),
                Message::Assistant(assistant) => formatter.emit_assistant(assistant),
            }
        }

        formatter.emit_complete();
        Ok(formatter.output)
    }

    fn complete(_last: Message, messages: &[Message]) -> anyhow::Result<String> {
        let mut formatter = Llama3::default();
        for message in messages {
            match message {
                Message::System(system) => formatter.emit_system(system),
                Message::User(user) => formatter.emit_user(user),
                Message::Assistant(assistant) => formatter.emit_assistant(assistant),
            }
        }
        formatter.emit_complete();
        Ok(formatter.output)
    }
}
