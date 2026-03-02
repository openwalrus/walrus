//! Stateful agent execution unit.
//!
//! [`Agent`] owns its configuration, message history, and event sender.
//! It drives LLM execution through [`Agent::step`], [`Agent::run`], and
//! [`Agent::run_stream`].

use crate::dispatch::Dispatcher;
use crate::event::{AgentEvent, AgentResponse, AgentStep, AgentStopReason};
use crate::model::{Message, Model, Request};
use anyhow::Result;
use async_stream::stream;
use futures_core::Stream;
use tokio::sync::mpsc;

pub use builder::AgentBuilder;
pub use config::AgentConfig;

mod builder;
pub mod config;

/// A stateful agent execution unit.
///
/// Holds configuration, conversation history, and an event sender.
/// Callers drive execution via `step()` (single LLM round), `run()`
/// (loop to completion), or `run_stream()` (yields events as a stream).
pub struct Agent {
    /// Agent configuration (name, prompt, model, limits, tool_choice).
    pub config: AgentConfig,
    /// Conversation history (system prompt + user/assistant/tool messages).
    history: Vec<Message>,
    /// Event sender for real-time status reporting.
    event_tx: mpsc::Sender<AgentEvent>,
}

impl Agent {
    /// Push a message into the conversation history.
    pub fn push_message(&mut self, message: Message) {
        self.history.push(message);
    }

    /// Return a reference to the conversation history.
    pub fn messages(&self) -> &[Message] {
        &self.history
    }

    /// Perform a single LLM round: send request, dispatch tools, return step.
    ///
    /// Composes a [`Request`] from config state (system prompt + history +
    /// dispatcher tools), calls `model.send()`, dispatches any tool calls
    /// via `dispatcher.dispatch()`, and appends results to history.
    ///
    /// This is a pure computation — no events are emitted. Callers (`run`
    /// and `run_stream`) handle event emission.
    pub async fn step<M: Model, D: Dispatcher>(
        &mut self,
        model: &M,
        dispatcher: &D,
    ) -> Result<AgentStep> {
        let model_name = self
            .config
            .model
            .clone()
            .unwrap_or_else(|| model.active_model());

        let mut messages = Vec::with_capacity(1 + self.history.len());
        if !self.config.system_prompt.is_empty() {
            messages.push(Message::system(&self.config.system_prompt));
        }
        messages.extend(self.history.iter().cloned());

        let tools = dispatcher.tools();
        let mut request = Request::new(model_name)
            .with_messages(messages)
            .with_tool_choice(self.config.tool_choice.clone());
        if !tools.is_empty() {
            request = request.with_tools(tools);
        }

        let response = model.send(&request).await?;
        let tool_calls = response.tool_calls().unwrap_or_default().to_vec();

        // Append the assistant message to history.
        if let Some(msg) = response.message() {
            self.history.push(msg);
        }

        // Dispatch tool calls if any.
        let mut tool_results = Vec::new();
        if !tool_calls.is_empty() {
            let calls: Vec<(&str, &str)> = tool_calls
                .iter()
                .map(|tc| (tc.function.name.as_str(), tc.function.arguments.as_str()))
                .collect();

            let results = dispatcher.dispatch(&calls).await;

            for (tc, result) in tool_calls.iter().zip(results) {
                let output = match result {
                    Ok(s) => s,
                    Err(e) => format!("error: {e}"),
                };

                let msg = Message::tool(&output, tc.id.clone());
                self.history.push(msg.clone());
                tool_results.push(msg);
            }
        }

        Ok(AgentStep {
            response,
            tool_calls,
            tool_results,
        })
    }

    /// Emit events for a completed step through the event channel.
    async fn emit_step_events(&self, step: &AgentStep) {
        if let Some(text) = step.response.content() {
            let _ = self
                .event_tx
                .send(AgentEvent::TextDelta(text.clone()))
                .await;
        }

        if !step.tool_calls.is_empty() {
            let _ = self
                .event_tx
                .send(AgentEvent::ToolCallsStart(step.tool_calls.clone()))
                .await;

            for (tc, result) in step.tool_calls.iter().zip(&step.tool_results) {
                let _ = self
                    .event_tx
                    .send(AgentEvent::ToolResult {
                        call_id: tc.id.clone(),
                        output: result.content.clone(),
                    })
                    .await;
            }

            let _ = self.event_tx.send(AgentEvent::ToolCallsComplete).await;
        }
    }

    /// Determine the stop reason for a step with no tool calls.
    fn stop_reason(step: &AgentStep) -> AgentStopReason {
        if step.response.content().is_some() {
            AgentStopReason::TextResponse
        } else {
            AgentStopReason::NoAction
        }
    }

    /// Run the agent loop up to `max_iterations`, returning the final response.
    ///
    /// Each iteration calls [`Agent::step`]. Stops when the model produces a
    /// response with no tool calls, hits the iteration limit, or errors.
    /// Events are emitted through the channel.
    pub async fn run<M: Model, D: Dispatcher>(
        &mut self,
        model: &M,
        dispatcher: &D,
    ) -> AgentResponse {
        let mut steps = Vec::new();
        let max = self.config.max_iterations;

        for _ in 0..max {
            match self.step(model, dispatcher).await {
                Ok(step) => {
                    let has_tool_calls = !step.tool_calls.is_empty();
                    let text = step.response.content().cloned();
                    self.emit_step_events(&step).await;

                    if !has_tool_calls {
                        let stop_reason = Self::stop_reason(&step);
                        steps.push(step);
                        let response = AgentResponse {
                            final_response: text,
                            iterations: steps.len(),
                            stop_reason,
                            steps,
                        };
                        let _ = self.event_tx.send(AgentEvent::Done(response.clone())).await;
                        return response;
                    }

                    steps.push(step);
                }
                Err(e) => {
                    let response = AgentResponse {
                        final_response: None,
                        iterations: steps.len(),
                        stop_reason: AgentStopReason::Error(e.to_string()),
                        steps,
                    };
                    let _ = self.event_tx.send(AgentEvent::Done(response.clone())).await;
                    return response;
                }
            }
        }

        let final_response = steps.last().and_then(|s| s.response.content().cloned());
        let response = AgentResponse {
            final_response,
            iterations: steps.len(),
            stop_reason: AgentStopReason::MaxIterations,
            steps,
        };
        let _ = self.event_tx.send(AgentEvent::Done(response.clone())).await;
        response
    }

    /// Run the agent loop as a stream of [`AgentEvent`]s.
    ///
    /// Yields events as they are produced during execution. This is a
    /// convenience wrapper that calls [`Agent::step`] in a loop and yields
    /// events directly. Events are not emitted through the channel.
    pub fn run_stream<'a, M: Model + 'a, D: Dispatcher + 'a>(
        &'a mut self,
        model: &'a M,
        dispatcher: &'a D,
    ) -> impl Stream<Item = AgentEvent> + 'a {
        stream! {
            let mut steps = Vec::new();
            let max = self.config.max_iterations;

            for _ in 0..max {
                match self.step(model, dispatcher).await {
                    Ok(step) => {
                        let has_tool_calls = !step.tool_calls.is_empty();
                        let text = step.response.content().cloned();

                        if let Some(ref t) = text {
                            yield AgentEvent::TextDelta(t.clone());
                        }

                        if has_tool_calls {
                            yield AgentEvent::ToolCallsStart(step.tool_calls.clone());
                            for (tc, result) in step.tool_calls.iter().zip(&step.tool_results) {
                                yield AgentEvent::ToolResult {
                                    call_id: tc.id.clone(),
                                    output: result.content.clone(),
                                };
                            }
                            yield AgentEvent::ToolCallsComplete;
                        }

                        if !has_tool_calls {
                            let stop_reason = Self::stop_reason(&step);
                            steps.push(step);
                            let response = AgentResponse {
                                final_response: text,
                                iterations: steps.len(),
                                stop_reason,
                                steps,
                            };
                            yield AgentEvent::Done(response);
                            return;
                        }

                        steps.push(step);
                    }
                    Err(e) => {
                        let response = AgentResponse {
                            final_response: None,
                            iterations: steps.len(),
                            stop_reason: AgentStopReason::Error(e.to_string()),
                            steps,
                        };
                        yield AgentEvent::Done(response);
                        return;
                    }
                }
            }

            let final_response = steps.last().and_then(|s| s.response.content().cloned());
            let response = AgentResponse {
                final_response,
                iterations: steps.len(),
                stop_reason: AgentStopReason::MaxIterations,
                steps,
            };
            yield AgentEvent::Done(response);
        }
    }
}
