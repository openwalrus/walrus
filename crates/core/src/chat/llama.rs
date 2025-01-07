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
            .push_str("You are a helpful assistant.<|eot_id|>\n");
    }

    fn emit_system(&mut self, system: &str) {
        self.output.push_str(&format!(
            "<|start_header_id|>system<|end_header_id|>\n{system}<|eot_id|>\n"
        ));
    }

    fn emit_user(&mut self, user: &str) {
        self.output.push_str(&format!(
            "<|start_header_id|>user<|end_header_id|>\n{user}<|eot_id|>\n"
        ));
    }

    fn emit_assistant(&mut self, assistant: &str) {
        self.output.push_str(&format!(
            "<|start_header_id|>assistant<|end_header_id|>\n{assistant}<|eot_id|>\n"
        ));
    }

    fn emit_eot(&mut self) {
        self.output.push_str("<|eot_id|>\n");
    }

    fn emit_complete(&mut self) {
        self.output
            .push_str("<|start_header_id|>assistant<|end_header_id|>\n");
    }
}

impl Formatter for Llama3 {
    const EOS_TOKEN: &str = "<|end_of_text|>";

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
        println!("{}", formatter.output);
        Ok(formatter.output)
    }

    fn complete(last: Message, messages: &[Message]) -> anyhow::Result<String> {
        let mut formatter = Llama3::default();
        formatter.output.push_str(last.text());
        formatter.emit_eot();

        for message in messages {
            match message {
                Message::System(system) => formatter.emit_system(system),
                Message::User(user) => formatter.emit_user(user),
                Message::Assistant(assistant) => formatter.emit_assistant(assistant),
            }
        }
        formatter.emit_complete();
        println!("{}", formatter.output);
        Ok(formatter.output)
    }
}
