//! Chat abstractions for the unified LLM Interfaces

use crate::{
    Agent, Config, FinishReason, General, InMemory, LLM, Memory, Response, Role, StreamChunk,
    ToolChoice,
    message::Message,
};
use anyhow::Result;
use futures_core::Stream;
use futures_util::StreamExt;

const MAX_TOOL_CALLS: usize = 16;

/// A chat for the LLM
#[derive(Clone)]
pub struct Chat<P: LLM, A: Agent, M: Memory = InMemory> {
    /// The chat configuration
    pub config: P::ChatConfig,

    /// Conversation memory
    pub memory: M,

    /// The LLM provider
    provider: P,

    /// The agent
    agent: A,

    /// Whether to return the usage information in stream mode
    usage: bool,
}

impl<P: LLM> Chat<P, (), InMemory> {
    /// Create a new chat
    pub fn new(config: General, provider: P) -> Self {
        Self {
            memory: InMemory::default(),
            provider,
            usage: config.usage,
            agent: (),
            config: config.into(),
        }
    }
}

impl<P: LLM, A: Agent, M: Memory> Chat<P, A, M> {
    /// Create a chat with a custom memory backend.
    pub fn with_memory(config: General, provider: P, agent: A, memory: M) -> Self {
        let usage = config.usage;
        let config_typed: P::ChatConfig = config.into();
        let config_typed = config_typed.with_tools(A::tools());
        Self {
            memory,
            provider,
            usage,
            agent,
            config: config_typed,
        }
    }

    /// Get a mutable reference to the agent
    pub fn agent_mut(&mut self) -> &mut A {
        &mut self.agent
    }

    /// Get the chat messages for API requests.
    ///
    /// Loads from memory, prepends the system prompt, applies agent-specific
    /// compaction, then strips reasoning content from non-tool-call messages.
    pub async fn messages(&self) -> Result<Vec<Message>> {
        let mut messages = self.memory.load().await?;

        // Ensure system prompt is always first
        if messages.first().map(|m| m.role) != Some(Role::System) {
            messages.insert(0, Message::system(A::SYSTEM_PROMPT));
        }

        Ok(self
            .agent
            .compact(messages)
            .into_iter()
            .map(|mut m| {
                if m.tool_calls.is_empty() {
                    m.reasoning_content = String::new();
                }
                m
            })
            .collect())
    }

    /// Set the agent and configure tools.
    ///
    /// The system prompt is not stored in memory — it is prepended
    /// dynamically by [`messages()`] before each LLM call.
    pub fn system<B: Agent>(self, agent: B) -> Chat<P, B, M> {
        let config = self.config.with_tools(B::tools());
        Chat {
            memory: self.memory,
            provider: self.provider,
            usage: self.usage,
            agent,
            config,
        }
    }

    /// Send a message to the LLM
    pub async fn send(&mut self, message: Message) -> Result<Response> {
        let mut config = self
            .config
            .with_tool_choice(self.agent.filter(message.content.as_str()));
        self.memory.append(&[message]).await?;

        for _ in 0..MAX_TOOL_CALLS {
            let messages = self.messages().await?;
            let response = self.provider.send(&config, &messages).await?;
            let Some(message) = response.message() else {
                return Ok(response);
            };

            if message.tool_calls.is_empty() {
                self.memory.append(&[message]).await?;
                return Ok(response);
            }

            let result = self.agent.dispatch(&message.tool_calls).await;
            let mut new_messages = vec![message];
            new_messages.extend(result);
            self.memory.append(&new_messages).await?;
            config = config.with_tool_choice(ToolChoice::None);
        }

        anyhow::bail!("max tool calls reached");
    }

    /// Send a message to the LLM with streaming
    pub fn stream(
        &mut self,
        message: Message,
    ) -> impl Stream<Item = Result<A::Chunk>> + use<'_, P, A, M> {
        let mut config = self
            .config
            .with_tool_choice(self.agent.filter(message.content.as_str()));

        async_stream::try_stream! {
            self.memory.append(&[message]).await?;

            for _ in 0..MAX_TOOL_CALLS {
                let messages = self.messages().await?;
                let mut builder = Message::builder(Role::Assistant);

                // Stream the chunks
                let inner = self.provider.stream(config.clone(), &messages, self.usage);
                futures_util::pin_mut!(inner);
                while let Some(result) = inner.next().await {
                    let chunk = match result {
                        Ok(chunk) => chunk,
                        Err(e) => {
                            tracing::error!("Error in LLM stream: {:?}", e);
                            Err(e)?
                        }
                    };

                    if builder.accept(&chunk) {
                        yield self.agent.chunk(&chunk).await?;
                    }

                    if let Some(reason) = chunk.reason() {
                        match reason {
                            FinishReason::Stop => return,
                            FinishReason::ToolCalls => break,
                            reason => Err(anyhow::anyhow!("unexpected finish reason: {reason:?}"))?,
                        }
                    }
                }

                // Build the message and dispatch tool calls
                let message = builder.build();
                if message.tool_calls.is_empty() {
                    self.memory.append(&[message]).await?;
                    break;
                }

                yield self.agent.chunk(&StreamChunk::tool(&message.tool_calls)).await?;
                let result = self.agent.dispatch(&message.tool_calls).await;
                let mut new_messages = vec![message];
                new_messages.extend(result);
                self.memory.append(&new_messages).await?;
                config = config.with_tool_choice(ToolChoice::None);
            }
        }
    }
}
