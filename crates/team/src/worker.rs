//! Type-erased worker registry entry.

use crate::local::Local;
use crate::protocol::Protocol;
use crate::task::{Task, TaskResult};
use anyhow::Result;
use ccore::{Agent, General, LLM, StreamChunk, Tool};
use schemars::Schema;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Object-safe protocol wrapper for type erasure.
trait ErasedProtocol: Send + Sync {
    fn call(&self, task: Task) -> Pin<Box<dyn Future<Output = Result<TaskResult>> + '_>>;
}

impl<P: Protocol> ErasedProtocol for P {
    fn call(&self, task: Task) -> Pin<Box<dyn Future<Output = Result<TaskResult>> + '_>> {
        Box::pin(Protocol::call(self, task))
    }
}

/// A registered worker in the team.
///
/// Holds agent metadata and a type-erased protocol handle.
/// The concrete agent/transport types are erased — the leader
/// interacts only through the protocol.
#[derive(Clone)]
pub struct Worker {
    name: String,
    description: String,
    parameters: Schema,
    handle: Arc<dyn ErasedProtocol>,
}

impl Worker {
    /// Create a worker from any protocol implementation.
    pub fn new<P: Protocol>(
        name: impl Into<String>,
        description: impl Into<String>,
        protocol: P,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: default_input_schema(),
            handle: Arc::new(protocol),
        }
    }

    /// Create a local (in-process) worker from an agent.
    ///
    /// Reads `name()` and `description()` from the agent's trait methods.
    pub fn local<P, A>(agent: A, provider: P, config: General) -> Self
    where
        P: LLM + Send + Sync + 'static,
        A: Agent<Chunk = StreamChunk> + Send + Sync + 'static,
    {
        Self {
            name: agent.name().into(),
            description: agent.description().into(),
            parameters: default_input_schema(),
            handle: Arc::new(Local::new(provider, config, agent)),
        }
    }

    /// The worker's tool name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Build a Tool definition for LLM registration.
    pub fn tool(&self) -> Tool {
        Tool {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
            strict: true,
        }
    }

    /// Call the worker with a task.
    pub async fn call(&self, task: Task) -> Result<TaskResult> {
        self.handle.call(task).await
    }
}

/// Default input for agent-as-tool calls.
#[derive(schemars::JsonSchema, serde::Deserialize)]
#[allow(dead_code)]
struct DefaultInput {
    /// The task or question to delegate to this agent.
    input: String,
}

fn default_input_schema() -> Schema {
    schemars::schema_for!(DefaultInput)
}
