//! Chat abstractions for the unified LLM Interfaces

use crate::{
    Agent, Config, FinishReason, General, LLM, Response, Role,
    message::{AssistantMessage, Message, ToolMessage},
};
use anyhow::Result;
use futures_core::Stream;
use futures_util::StreamExt;
use serde::Serialize;

const MAX_TOOL_CALLS: usize = 16;

/// A chat for the LLM
#[derive(Clone)]
pub struct Chat<P: LLM, A: Agent> {
    /// The chat configuration
    pub config: P::ChatConfig,

    /// Chat history in memory
    pub messages: Vec<ChatMessage>,

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
    /// Add the system prompt to the chat
    pub fn system<B: Agent>(mut self, agent: B) -> Chat<P, B> {
        let mut messages = self.messages;
        if messages.is_empty() {
            messages.push(Message::system(A::SYSTEM_PROMPT).into());
        } else if let Some(ChatMessage::System(_)) = messages.first() {
            messages.insert(0, Message::system(A::SYSTEM_PROMPT).into());
        } else {
            messages = vec![Message::system(A::SYSTEM_PROMPT).into()]
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
        self.messages.push(message.into());

        for _ in 0..MAX_TOOL_CALLS {
            let response = self.provider.send(&config, &self.messages).await?;
            let Some(tool_calls) = response.tool_calls() else {
                return Ok(response);
            };

            let result = self.agent.dispatch(tool_calls).await;
            self.messages.extend(result.into_iter().map(Into::into));
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
        self.messages.push(message.into());

        async_stream::try_stream! {
            for _ in 0..MAX_TOOL_CALLS {
                let messages = self.messages.clone();
                let inner = self.provider.stream(config.clone(), &messages, self.usage);
                futures_util::pin_mut!(inner);

                let mut tool_calls = None;
                let mut message = String::new();
                while let Some(chunk) = inner.next().await {
                    let chunk = chunk?;
                    if let Some(calls) = chunk.tool_calls() {
                        tool_calls = Some(calls.to_vec());
                    }

                    if let Some(content) = chunk.content() {
                        message.push_str(content);
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

                if !message.is_empty() {
                    self.messages.push(Message::assistant(&message).into());
                }

                if let Some(calls) = tool_calls {
                    let result = self.agent.dispatch(&calls).await;
                    self.messages.extend(result.into_iter().map(Into::into));
                } else {
                    break;
                }
            }

            Err(anyhow::anyhow!("max tool calls reached"))?;
        }
    }
}

/// A chat message in memory
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ChatMessage {
    /// A user message
    User(Message),

    /// An assistant message
    Assistant(AssistantMessage),

    /// A tool message
    Tool(ToolMessage),

    /// A system message
    System(Message),
}

impl From<Message> for ChatMessage {
    fn from(message: Message) -> Self {
        match message.role {
            Role::User => ChatMessage::User(message),
            Role::Assistant => ChatMessage::Assistant(AssistantMessage {
                message,
                prefix: false,
                reasoning: String::new(),
            }),
            Role::System => ChatMessage::System(message),
            Role::Tool => ChatMessage::Tool(ToolMessage {
                tool: String::new(),
                message,
            }),
        }
    }
}

impl From<ToolMessage> for ChatMessage {
    fn from(message: ToolMessage) -> Self {
        ChatMessage::Tool(message)
    }
}
