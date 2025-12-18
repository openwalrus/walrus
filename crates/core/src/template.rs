//! Turbofish Agent library

use crate::{Message, Role};
use serde::{Deserialize, Serialize};

/// A template of the system prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// The system prompt for the agent
    pub system: String,

    /// The input example
    pub input: String,

    /// The output json example
    pub output: String,
}

impl Template {
    /// Create a new message from the template
    pub fn message(&self) -> Message {
        Message {
            content: format!(
                r#"{}
                
                EXAMPLE INPUT:
                {}
                
                EXAMPLE JSON OUTPUT:
                {}"#,
                self.system, self.input, self.output
            ),
            name: String::new(),
            role: Role::System,
        }
    }
}
