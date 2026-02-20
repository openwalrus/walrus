//! Team composition: merge agent configs and register forwarding in runtime.
//!
//! A team is just a function that:
//! 1. Creates a tool definition for each worker agent.
//! 2. Registers forwarding handlers in the runtime.
//! 3. Adds the worker tool names to the leader's tool list.
//!
//! # Example
//!
//! ```rust,ignore
//! use cydonia_agent::{Agent, build_team, Chat, Provider};
//! use runtime::Runtime;
//!
//! let leader = Agent::new("leader").system_prompt("You coordinate.");
//! let analyst = Agent::new("analyst")
//!     .description("Market analysis")
//!     .system_prompt("You analyze markets.");
//!
//! let mut runtime = Runtime::new();
//! let leader = build_team(leader, vec![analyst], &mut runtime);
//! ```

use crate::Agent;
use anyhow::Result;
use llm::Tool;
use runtime::Runtime;
use schemars::JsonSchema;
use serde::Deserialize;

/// Build a team: register each worker as a tool and add to the leader.
///
/// Each worker becomes a tool with a standard `{ input: string }` schema.
/// The handler is a placeholder that returns the input — real dispatch
/// is left to the caller's runtime setup (e.g., creating sub-chats).
pub fn build_team(
    mut leader: Agent,
    workers: Vec<Agent>,
    runtime: &mut Runtime,
) -> Agent {
    for worker in &workers {
        let tool_def = worker_tool(&worker.name, &worker.description);
        // Register a placeholder handler — real forwarding is wired
        // by the caller based on their specific architecture.
        let name = worker.name.clone();
        runtime.register(tool_def, move |input| {
            let name = name.clone();
            async move {
                format!("[{name}] received: {input}")
            }
        });
        leader.tools.push(worker.name.clone());
    }
    leader
}

/// Build a tool definition for a worker agent.
///
/// Uses a standard `{ input: string }` schema so the leader
/// can delegate tasks with a single text field.
pub fn worker_tool(name: impl Into<String>, description: impl Into<String>) -> Tool {
    Tool {
        name: name.into(),
        description: description.into(),
        parameters: default_input_schema(),
        strict: true,
    }
}

/// Extract the `input` field from tool call arguments JSON.
pub fn extract_input(arguments: &str) -> Result<String> {
    let parsed: serde_json::Value = serde_json::from_str(arguments)?;
    parsed
        .get("input")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("missing 'input' field in arguments"))
}

/// Default input schema for agent-as-tool calls.
#[derive(JsonSchema, Deserialize)]
#[allow(dead_code)]
struct DefaultInput {
    /// The task or question to delegate to this agent.
    input: String,
}

fn default_input_schema() -> schemars::Schema {
    schemars::schema_for!(DefaultInput)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_input_parses_json() {
        let json = r#"{"input": "analyze BTC"}"#;
        assert_eq!(extract_input(json).unwrap(), "analyze BTC");
    }

    #[test]
    fn extract_input_fails_on_missing_field() {
        let json = r#"{"query": "analyze BTC"}"#;
        assert!(extract_input(json).is_err());
    }

    #[test]
    fn extract_input_fails_on_invalid_json() {
        assert!(extract_input("not json").is_err());
    }

    #[test]
    fn worker_tool_builds_tool() {
        let t = worker_tool("analyst", "market analysis");
        assert_eq!(t.name, "analyst");
        assert_eq!(t.description, "market analysis");
        assert!(t.strict);
        let json = serde_json::to_string(&t.parameters).unwrap();
        assert!(json.contains("input"));
    }

    #[test]
    fn build_team_adds_worker_tools() {
        let leader = Agent::new("leader")
            .system_prompt("I coordinate.")
            .tool("search");
        let analyst = Agent::new("analyst").description("market analysis");

        let mut rt = Runtime::new();
        let leader = build_team(leader, vec![analyst], &mut rt);

        assert_eq!(leader.tools.len(), 2);
        assert_eq!(leader.tools[0], "search");
        assert_eq!(leader.tools[1], "analyst");

        // Runtime should have the analyst tool registered
        let tools = rt.resolve(&["analyst".into()]);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "analyst");
    }
}
