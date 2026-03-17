//! Tool schemas and dispatch for built-in memory tools.

use crate::hook::DaemonHook;
use serde::Deserialize;
use wcore::{
    agent::{AsTool, ToolDescription},
    model::Tool,
};

#[derive(Deserialize, schemars::JsonSchema)]
pub(crate) struct Recall {
    /// Keyword or phrase to search your persistent memory for.
    pub query: String,
}

impl ToolDescription for Recall {
    const DESCRIPTION: &'static str =
        "Search your persistent memory (notes, user profile, facts) by keyword.";
}

#[derive(Deserialize, schemars::JsonSchema)]
pub(crate) struct Memory {
    /// Important information to save to persistent memory.
    pub content: String,
}

impl ToolDescription for Memory {
    const DESCRIPTION: &'static str =
        "Write important information to your persistent memory. Persists across sessions.";
}

#[derive(Deserialize, schemars::JsonSchema)]
pub(crate) struct UserMemory {
    /// User profile information to save.
    pub content: String,
}

impl ToolDescription for UserMemory {
    const DESCRIPTION: &'static str =
        "Write user profile information to persistent memory. Overwrites existing user profile.";
}

pub(crate) fn tools() -> Vec<Tool> {
    vec![Recall::as_tool(), Memory::as_tool(), UserMemory::as_tool()]
}

impl DaemonHook {
    pub(crate) async fn dispatch_recall(&self, args: &str) -> String {
        let input: Recall = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        match self.memory {
            Some(ref mem) => mem.recall(&input.query),
            None => "memory not available".to_owned(),
        }
    }

    pub(crate) async fn dispatch_memory(&self, args: &str) -> String {
        let input: Memory = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        match self.memory {
            Some(ref mem) => mem.write_memory(&input.content),
            None => "memory not available".to_owned(),
        }
    }

    pub(crate) async fn dispatch_user_memory(&self, args: &str) -> String {
        let input: UserMemory = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        match self.memory {
            Some(ref mem) => mem.write_user(&input.content),
            None => "memory not available".to_owned(),
        }
    }
}
