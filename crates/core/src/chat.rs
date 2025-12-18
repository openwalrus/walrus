//! Chat abstractions for the unified LLM Interfaces

use crate::{
    Agent, Config, FinishReason, General, LLM, Response, Role, ToolCall, message::Message,
};
use anyhow::Result;
use futures_core::Stream;
use futures_util::StreamExt;
use std::collections::HashMap;

const MAX_TOOL_CALLS: usize = 16;

/// A chat for the LLM
#[derive(Clone)]
pub struct Chat<P: LLM, A: Agent> {
    /// The chat configuration
    pub config: P::ChatConfig,

    /// Chat history in memory
    pub messages: Vec<Message>,

    /// The LLM provider
    provider: P,

    /// The agent
    agent: A,

    /// Whether to return the usage information in stream mode
    usage: bool,
}

impl<P: LLM> Chat<P, ()> {
    /// Create a new chat
    pub fn new(config: General, provider: P) -> Self {
        Self {
            messages: vec![],
            provider,
            usage: config.usage,
            agent: (),
            config: config.into(),
        }
    }
}

impl<P: LLM, A: Agent> Chat<P, A> {
    /// Get the chat messages
    pub fn messages(&self) -> Vec<Message> {
        self.messages
            .clone()
            .into_iter()
            .map(|m| {
                // m.reasoning_content = String::new();
                m
            })
            .collect()
    }

    /// Add the system prompt to the chat
    pub fn system<B: Agent>(mut self, agent: B) -> Chat<P, B> {
        let mut messages = self.messages;
        if messages.is_empty() {
            messages.push(Message::system(B::SYSTEM_PROMPT));
        } else if messages.first().map(|m| m.role) == Some(Role::System) {
            messages.insert(0, Message::system(B::SYSTEM_PROMPT));
        } else {
            messages = vec![Message::system(B::SYSTEM_PROMPT)]
                .into_iter()
                .chain(messages)
                .collect();
        }

        self.config = self.config.with_tools(A::tools());
        Chat {
            messages,
            provider: self.provider,
            usage: self.usage,
            agent,
            config: self.config,
        }
    }

    /// Send a message to the LLM
    pub async fn send(&mut self, message: Message) -> Result<Response> {
        let config = self
            .config
            .with_tool_choice(self.agent.filter(message.content.as_str()));
        self.messages.push(message);

        for _ in 0..MAX_TOOL_CALLS {
            let response = self.provider.send(&config, &self.messages).await?;
            if let Some(message) = response.message() {
                self.messages.push(Message::assistant(
                    message,
                    response.reasoning().cloned(),
                    response.tool_calls(),
                ));
            }

            let Some(tool_calls) = response.tool_calls() else {
                return Ok(response);
            };

            let result = self.agent.dispatch(tool_calls).await;
            self.messages.extend(result);
        }

        anyhow::bail!("max tool calls reached");
    }

    /// Send a message to the LLM with streaming
    pub fn stream(
        &mut self,
        message: Message,
    ) -> impl Stream<Item = Result<A::Chunk>> + use<'_, P, A> {
        let config = self
            .config
            .with_tool_choice(self.agent.filter(message.content.as_str()));
        self.messages.push(message);

        async_stream::try_stream! {
            for _ in 0..MAX_TOOL_CALLS {
                let messages = self.messages();
                let inner = self.provider.stream(config.clone(), &messages, self.usage);
                futures_util::pin_mut!(inner);

                let mut tool_calls: HashMap<u32, ToolCall> = HashMap::new();
                let mut message = String::new();
                let mut reasoning = String::new();
                while let Some(chunk) = inner.next().await {
                    let chunk = chunk?;
                    if let Some(calls) = chunk.tool_calls() {
                        for call in calls {
                            let entry = tool_calls.entry(call.index).or_default();
                            if !call.id.is_empty() {
                                entry.id.clone_from(&call.id);
                            }
                            if !call.call_type.is_empty() {
                                entry.call_type.clone_from(&call.call_type);
                            }
                            if !call.function.name.is_empty() {
                                entry.function.name.clone_from(&call.function.name);
                            }
                            entry.function.arguments.push_str(&call.function.arguments);
                        }
                    }

                    if let Some(content) = chunk.content() {
                        message.push_str(content);
                    }

                    if let Some(reason) = chunk.reasoning_content() {
                        reasoning.push_str(reason);
                    }

                    yield self.agent.chunk(&chunk).await?;
                    if let Some(reason) = chunk.reason() {
                        match reason {
                            FinishReason::Stop => return,
                            FinishReason::ToolCalls => break,
                            reason => Err(anyhow::anyhow!("unexpected finish reason: {reason:?}"))?,
                        }
                    }
                }

                let reasoning = if reasoning.is_empty() { None } else { Some(reasoning) };
                if tool_calls.is_empty() {
                    self.messages.push(Message::assistant(&message, reasoning, None));
                    break;
                } else {
                    let mut calls: Vec<_> = tool_calls.into_values().collect();
                    calls.sort_by_key(|c| c.index);
                    self.messages.push(Message::assistant(&message, reasoning, Some(&calls)));
                    let result = self.agent.dispatch(&calls).await;
                    self.messages.extend(result);
                }
            }
        }
    }
}
