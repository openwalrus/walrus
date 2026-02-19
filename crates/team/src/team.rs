//! Team: a leader agent with a dynamic registry of workers.

use crate::task::Task;
use crate::worker::Worker;
use anyhow::Result;
use ccore::{Agent, Message, StreamChunk, Tool, ToolCall};
use std::collections::BTreeMap;

/// Extract the `input` field from tool call arguments JSON.
fn extract_input(arguments: &str) -> Result<String> {
    let parsed: serde_json::Value = serde_json::from_str(arguments)?;
    parsed
        .get("input")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("missing 'input' field in arguments"))
}

/// A team of agents: one leader + dynamic workers.
///
/// Implements [`Agent`] so it composes directly with [`Chat`](ccore::Chat).
/// The leader handles the conversation. Workers are registered
/// as tools the LLM can call. Workers can join or leave at runtime.
///
/// # Example
///
/// ```rust,ignore
/// use cydonia_team::{Team, Worker};
///
/// let mut team = Team::new(leader_agent);
/// team.register(Worker::local(analyst, provider.clone(), config.clone()));
/// team.register(Worker::local(risk, provider.clone(), config.clone()));
/// let chat = Chat::new(config, provider, team, vec![]);
/// ```
#[derive(Clone)]
pub struct Team<A: Agent> {
    /// The leader agent.
    leader: A,
    /// Registered workers keyed by name.
    workers: BTreeMap<String, Worker>,
}

impl<A: Agent> Team<A> {
    /// Create a new team with the given leader agent.
    pub fn new(leader: A) -> Self {
        Self {
            leader,
            workers: BTreeMap::new(),
        }
    }

    /// Register a worker. Returns the previous worker if the name existed.
    pub fn register(&mut self, worker: Worker) -> Option<Worker> {
        self.workers.insert(worker.name().into(), worker)
    }

    /// Remove a worker by name. Returns the removed worker.
    pub fn remove(&mut self, name: &str) -> Option<Worker> {
        self.workers.remove(name)
    }

    /// List registered worker names.
    pub fn workers(&self) -> impl Iterator<Item = &str> {
        self.workers.keys().map(|s| s.as_str())
    }
}

impl<A: Agent> Agent for Team<A> {
    type Chunk = A::Chunk;

    fn name(&self) -> &str {
        self.leader.name()
    }

    fn description(&self) -> &str {
        self.leader.description()
    }

    fn system_prompt(&self) -> String {
        self.leader.system_prompt()
    }

    fn tools(&self) -> Vec<Tool> {
        let mut tools = self.leader.tools();
        tools.extend(self.workers.values().map(|w| w.tool()));
        tools
    }

    fn compact(&self, messages: Vec<Message>) -> Vec<Message> {
        self.leader.compact(messages)
    }

    async fn dispatch(&self, tools: &[ToolCall]) -> Vec<Message> {
        let mut results = Vec::new();
        let mut regular = Vec::new();

        for call in tools {
            if let Some(worker) = self.workers.get(&call.function.name) {
                match extract_input(&call.function.arguments) {
                    Ok(input) => match worker.call(Task { input }).await {
                        Ok(result) => {
                            results.push(Message::tool(result.output, &call.id));
                        }
                        Err(e) => {
                            results.push(Message::tool(
                                format!("worker error: {e}"),
                                &call.id,
                            ));
                        }
                    },
                    Err(e) => {
                        results.push(Message::tool(
                            format!("failed to parse arguments: {e}"),
                            &call.id,
                        ));
                    }
                }
            } else {
                regular.push(call.clone());
            }
        }

        if !regular.is_empty() {
            results.extend(self.leader.dispatch(&regular).await);
        }

        results
    }

    async fn chunk(&self, chunk: &StreamChunk) -> Result<Self::Chunk> {
        self.leader.chunk(chunk).await
    }
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
    fn register_and_tools() {
        use crate::protocol::Protocol;
        use crate::task::{TaskResult, Task};

        #[derive(Clone)]
        struct MockProtocol;

        impl Protocol for MockProtocol {
            async fn call(&self, task: Task) -> Result<TaskResult> {
                Ok(TaskResult {
                    output: format!("echo: {}", task.input),
                })
            }
        }

        let mut team = Team::new(());
        assert!(team.tools().is_empty());

        team.register(Worker::new("analyst", "market analysis", MockProtocol));
        let tools = team.tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "analyst");
    }

    #[test]
    fn register_and_remove() {
        use crate::protocol::Protocol;
        use crate::task::{TaskResult, Task};

        #[derive(Clone)]
        struct MockProtocol;

        impl Protocol for MockProtocol {
            async fn call(&self, task: Task) -> Result<TaskResult> {
                Ok(TaskResult {
                    output: format!("echo: {}", task.input),
                })
            }
        }

        let mut team = Team::new(());
        team.register(Worker::new("analyst", "market analysis", MockProtocol));
        team.register(Worker::new("risk", "risk assessment", MockProtocol));
        assert_eq!(team.tools().len(), 2);

        let removed = team.remove("analyst");
        assert!(removed.is_some());
        let tools = team.tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "risk");
    }

    #[test]
    fn default_schema_has_input_field() {
        use crate::protocol::Protocol;
        use crate::task::{TaskResult, Task};

        #[derive(Clone)]
        struct MockProtocol;

        impl Protocol for MockProtocol {
            async fn call(&self, task: Task) -> Result<TaskResult> {
                Ok(TaskResult {
                    output: format!("echo: {}", task.input),
                })
            }
        }

        let mut team = Team::new(());
        team.register(Worker::new("test", "test worker", MockProtocol));
        let tool = &team.tools()[0];
        let json = serde_json::to_string(&tool.parameters).unwrap();
        assert!(json.contains("input"));
    }
}
