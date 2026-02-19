//! Sub-agent abstraction: an agent callable as a tool.

use crate::layer::{CALL_DEPTH, MAX_DEPTH};
use anyhow::Result;
use ccore::{Agent, Chat, General, LLM, Message, Tool};
use schemars::Schema;

/// A sub-agent that can be called as a tool by a parent agent.
///
/// Unlike [`Agent`] (which manages LLM conversations), `SubAgent`
/// represents a callable unit: text in → text out. Implementations
/// typically create a [`Chat::send()`] internally.
pub trait SubAgent: Clone {
    /// The tool name exposed to the parent agent's LLM.
    fn name(&self) -> &str;

    /// Human-readable description for the LLM.
    fn description(&self) -> &str;

    /// JSON Schema for the tool parameters.
    ///
    /// Default: a schema with a single required `input` string field.
    fn parameters(&self) -> Schema {
        default_input_schema()
    }

    /// Call the sub-agent with the given input text.
    ///
    /// Returns the final text response from the sub-agent's conversation.
    fn call(&self, input: &str) -> impl Future<Output = Result<String>>;

    /// Convert this sub-agent into a [`Tool`] definition for LLM registration.
    fn tool(&self) -> Tool {
        Tool {
            name: self.name().into(),
            description: self.description().into(),
            parameters: self.parameters(),
            strict: true,
        }
    }
}

/// Wraps any [`Agent`] + [`LLM`] pair into a [`SubAgent`].
///
/// Fully generic, zero-cost. The sub-agent runs its own [`Chat::send()`]
/// with a fresh message history on each call.
///
/// # Example
///
/// ```rust,ignore
/// let analyst = AgentSub::new(
///     "analyst",
///     "Technical market analysis",
///     provider.clone(),
///     config.clone(),
///     AnalystAgent::new(),
/// );
/// ```
#[derive(Clone)]
pub struct AgentSub<P: LLM, A: Agent> {
    name: String,
    description: String,
    provider: P,
    config: General,
    agent: A,
}

impl<P: LLM, A: Agent> AgentSub<P, A> {
    /// Create a new agent-backed sub-agent.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        provider: P,
        config: General,
        agent: A,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            provider,
            config,
            agent,
        }
    }
}

impl<P, A> SubAgent for AgentSub<P, A>
where
    P: LLM + Clone,
    A: Agent + Clone + ccore::Tools,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn call(&self, input: &str) -> Result<String> {
        let depth = CALL_DEPTH.try_with(|d| *d).unwrap_or(0);
        if depth >= MAX_DEPTH {
            anyhow::bail!("agent call depth limit reached ({MAX_DEPTH})");
        }

        let input = input.to_string();
        CALL_DEPTH
            .scope(depth + 1, async {
                let mut chat = Chat::with_tools(
                    self.config.clone(),
                    self.provider.clone(),
                    self.agent.clone(),
                    vec![],
                );
                let resp = chat.send(Message::user(input)).await?;
                Ok(resp.content().cloned().unwrap_or_default())
            })
            .await
    }
}

/// Default input for agent-as-tool calls.
#[derive(schemars::JsonSchema, serde::Deserialize)]
#[allow(dead_code)]
struct DefaultInput {
    /// The task or question to delegate to this agent.
    input: String,
}

/// Default schema: `{ "input": string }` with `input` required.
fn default_input_schema() -> Schema {
    schemars::schema_for!(DefaultInput)
}

/// Extract the `input` field from tool call arguments JSON.
pub(crate) fn extract_input(arguments: &str) -> Result<String> {
    let parsed: serde_json::Value = serde_json::from_str(arguments)?;
    parsed
        .get("input")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("missing 'input' field in arguments"))
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
    fn default_schema_has_input_field() {
        let schema = default_input_schema();
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("input"));
    }
}
