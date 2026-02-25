//! Team composition: register workers as tools in the runtime.
//!
//! # Example
//!
//! ```rust,ignore
//! use walrus_core::Agent;
//! use walrus_runtime::{Runtime, build_team};
//!
//! let leader = Agent::new("leader").system_prompt("You coordinate.");
//! let analyst = Agent::new("analyst").description("Market analysis");
//!
//! let leader = build_team(leader, vec![analyst], &mut runtime);
//! runtime.add_agent(leader);
//! ```

use agent::Agent;
use anyhow::Result;
use llm::Tool;

/// Build a team: register each worker as a tool and add to the leader.
pub fn build_team(mut leader: Agent, workers: Vec<Agent>, runtime: &mut crate::Runtime) -> Agent {
    for worker in &workers {
        let tool_def = worker_tool(&worker.name, &worker.description);
        let name = worker.name.clone();
        runtime.register(tool_def, move |input| {
            let name = name.clone();
            async move { format!("[{name}] received: {input}") }
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
#[derive(schemars::JsonSchema, serde::Deserialize)]
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
}
