//! `WithTeam` agent decorator — adds a single sub-agent as a tool.

use crate::sub::{SubAgent, extract_input};
use anyhow::Result;
use ccore::{Agent, Message, StreamChunk, Tool, ToolCall};

/// Agent layer that adds a single sub-agent as a tool.
///
/// Each `WithTeam` wraps one sub-agent. For multiple sub-agents,
/// nest layers — the types compose at compile time:
///
/// ```text
/// WithTeam<WithTeam<PerpAgent, Analyst>, Risk>
/// ```
///
/// # Example
///
/// ```rust,ignore
/// let agent = PerpAgent::new(pool, &req);
/// let agent = WithTeam::new(agent, analyst);  // adds "analyst" tool
/// let agent = WithTeam::new(agent, risk);     // adds "risk" tool
/// ```
#[derive(Clone)]
pub struct WithTeam<A: Agent, S: SubAgent> {
    /// The inner (parent) agent.
    pub agent: A,
    /// The sub-agent exposed as a tool.
    pub sub: S,
}

impl<A: Agent, S: SubAgent> WithTeam<A, S> {
    /// Create a new team-enhanced agent.
    pub fn new(agent: A, sub: S) -> Self {
        Self { agent, sub }
    }
}

impl<A: Agent, S: SubAgent> Agent for WithTeam<A, S> {
    type Chunk = A::Chunk;

    fn system_prompt(&self) -> String {
        self.agent.system_prompt()
    }

    fn tools(&self) -> Vec<Tool> {
        let mut tools = self.agent.tools();
        tools.push(self.sub.tool());
        tools
    }

    fn compact(&self, messages: Vec<Message>) -> Vec<Message> {
        self.agent.compact(messages)
    }

    async fn dispatch(&self, tools: &[ToolCall]) -> Vec<Message> {
        let mut results = Vec::new();
        let mut regular = Vec::new();

        for tool in tools {
            if tool.function.name == self.sub.name() {
                let input = match extract_input(&tool.function.arguments) {
                    Ok(input) => input,
                    Err(e) => {
                        results.push(Message::tool(
                            format!("failed to parse arguments: {e}"),
                            tool.id.clone(),
                        ));
                        continue;
                    }
                };
                match self.sub.call(&input).await {
                    Ok(response) => {
                        results.push(Message::tool(response, tool.id.clone()));
                    }
                    Err(e) => {
                        results.push(Message::tool(
                            format!("sub-agent error: {e}"),
                            tool.id.clone(),
                        ));
                    }
                }
            } else {
                regular.push(tool.clone());
            }
        }

        if !regular.is_empty() {
            results.extend(self.agent.dispatch(&regular).await);
        }

        results
    }

    async fn chunk(&self, chunk: &StreamChunk) -> Result<Self::Chunk> {
        self.agent.chunk(chunk).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MockSub {
        name: &'static str,
    }

    impl SubAgent for MockSub {
        fn name(&self) -> &str {
            self.name
        }
        fn description(&self) -> &str {
            "mock sub-agent"
        }
        async fn call(&self, input: &str) -> Result<String> {
            Ok(format!("echo: {input}"))
        }
    }

    #[test]
    fn tools_includes_sub() {
        let team = WithTeam::new((), MockSub { name: "analyst" });
        let tools = team.tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "analyst");
    }

    #[test]
    fn tools_recursive_across_nested_layers() {
        let team = WithTeam::new((), MockSub { name: "analyst" });
        let team = WithTeam::new(team, MockSub { name: "risk" });
        let tools = team.tools();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "analyst");
        assert_eq!(tools[1].name, "risk");
    }
}
