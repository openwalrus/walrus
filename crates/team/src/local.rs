//! In-process protocol implementation.

use crate::protocol::Protocol;
use crate::task::{Task, TaskResult};
use anyhow::Result;
use ccore::{Agent, Chat, General, LLM, Message, StreamChunk};

tokio::task_local! {
    /// Current agent call depth in the nested call chain.
    pub(crate) static CALL_DEPTH: usize;
}

/// Maximum nesting depth for agent-as-tool calls.
pub(crate) const MAX_DEPTH: usize = 3;

/// In-process protocol that calls an agent directly.
///
/// Each `call()` spins up a fresh [`Chat::send()`] with an empty
/// message history. The agent, provider, and config are captured
/// at construction time.
pub struct Local<P: LLM, A: Agent> {
    provider: P,
    config: General,
    agent: A,
}

impl<P: LLM, A: Agent> Local<P, A> {
    /// Create a new local protocol handle.
    pub fn new(provider: P, config: General, agent: A) -> Self {
        Self {
            provider,
            config,
            agent,
        }
    }
}

impl<P: LLM, A: Agent> Clone for Local<P, A> {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            config: self.config.clone(),
            agent: self.agent.clone(),
        }
    }
}

impl<P, A> Protocol for Local<P, A>
where
    P: LLM + Send + Sync + 'static,
    A: Agent<Chunk = StreamChunk> + Send + Sync + 'static,
{
    async fn call(&self, task: Task) -> Result<TaskResult> {
        let depth = CALL_DEPTH.try_with(|d| *d).unwrap_or(0);
        if depth >= MAX_DEPTH {
            anyhow::bail!("agent call depth limit reached ({MAX_DEPTH})");
        }

        let input = task.input;
        CALL_DEPTH
            .scope(depth + 1, async {
                let mut chat = Chat::new(
                    self.config.clone(),
                    self.provider.clone(),
                    self.agent.clone(),
                    vec![],
                );
                let resp = chat.send(Message::user(input)).await?;
                Ok(TaskResult {
                    output: resp.content().cloned().unwrap_or_default(),
                })
            })
            .await
    }
}
