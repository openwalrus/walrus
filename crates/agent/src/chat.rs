//! Chat orchestration — no generics.
//!
//! [`Chat`] holds a [`Provider`], an [`Agent`], and a [`Runtime`].
//! Tool dispatch goes through the runtime; streaming returns
//! [`StreamChunk`] directly (the caller handles display).

use crate::{Agent, Provider};
use anyhow::Result;
use futures_core::Stream;
use futures_util::StreamExt;
use llm::{FinishReason, General, Message, Response, Role, StreamChunk, ToolChoice};
use runtime::Runtime;

const MAX_TOOL_CALLS: usize = 16;

/// A chat session: agent config + provider + runtime + history.
pub struct Chat {
    /// The agent configuration.
    pub agent: Agent,
    /// Conversation messages.
    pub messages: Vec<Message>,
    /// The tool runtime.
    pub runtime: Runtime,
    /// LLM provider.
    provider: Provider,
    /// LLM config.
    config: General,
}

impl Chat {
    /// Create a new chat session.
    pub fn new(
        config: General,
        provider: Provider,
        agent: Agent,
        runtime: Runtime,
    ) -> Self {
        Self {
            agent,
            messages: Vec::new(),
            runtime,
            provider,
            config,
        }
    }

    /// Build the message list for an API request.
    ///
    /// Prepends the system prompt, applies compaction,
    /// and strips reasoning content from non-tool-call messages.
    fn api_messages(&self) -> Vec<Message> {
        let mut messages = self.messages.clone();

        // Prepend system prompt (built at runtime, never stored)
        if messages.first().map(|m| m.role) != Some(Role::System) {
            messages.insert(0, Message::system(&self.agent.system_prompt));
        }

        self.runtime
            .compact(&self.agent.name, messages)
            .into_iter()
            .map(|mut m| {
                if m.tool_calls.is_empty() {
                    m.reasoning_content = String::new();
                }
                m
            })
            .collect()
    }

    /// Send a message to the LLM (non-streaming).
    pub async fn send(&mut self, message: Message) -> Result<Response> {
        let tools = self.runtime.resolve(&self.agent.tools);
        let mut tool_choice = ToolChoice::Auto;
        self.messages.push(message);

        for _ in 0..MAX_TOOL_CALLS {
            let messages = self.api_messages();
            let response = self
                .provider
                .send(&self.config, &tools, tool_choice.clone(), &messages)
                .await?;
            let Some(message) = response.message() else {
                return Ok(response);
            };

            if message.tool_calls.is_empty() {
                self.messages.push(message);
                return Ok(response);
            }

            let result = self.runtime.dispatch(&message.tool_calls).await;
            self.messages.push(message);
            self.messages.extend(result);
            tool_choice = ToolChoice::None;
        }

        anyhow::bail!("max tool calls reached");
    }

    /// Send a message to the LLM with streaming.
    ///
    /// Returns [`StreamChunk`] directly — the caller handles display.
    pub fn stream(
        &mut self,
        message: Message,
    ) -> impl Stream<Item = Result<StreamChunk>> + use<'_> {
        let tools = self.runtime.resolve(&self.agent.tools);

        async_stream::try_stream! {
            self.messages.push(message);
            let mut tool_choice = ToolChoice::Auto;

            for _ in 0..MAX_TOOL_CALLS {
                let messages = self.api_messages();
                let mut builder = Message::builder(Role::Assistant);

                let inner = self.provider.stream(
                    &self.config,
                    &tools,
                    tool_choice.clone(),
                    &messages,
                );
                futures_util::pin_mut!(inner);

                while let Some(result) = inner.next().await {
                    let chunk = match result {
                        Ok(chunk) => chunk,
                        Err(e) => {
                            tracing::error!("Error in LLM stream: {:?}", e);
                            Err(e)?
                        }
                    };

                    let reason = chunk.reason().cloned();

                    if builder.accept(&chunk) {
                        yield chunk;
                    }

                    if let Some(reason) = reason {
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
                    self.messages.push(message);
                    break;
                }

                let result = self.runtime.dispatch(&message.tool_calls).await;
                self.messages.push(message);
                self.messages.extend(result);
                tool_choice = ToolChoice::None;
            }
        }
    }
}
