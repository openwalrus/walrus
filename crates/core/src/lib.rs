//! Walrus agent library.
//!
//! - [`Agent`]: Stateful execution unit with step/run/run_stream.
//! - [`AgentBuilder`]: Fluent construction requiring event sender.
//! - [`AgentConfig`]: Serializable agent parameters.
//! - [`Dispatcher`]: Generic async trait for tool dispatch.
//! - [`model`]: Unified LLM interface types and traits.
//! - Agent event types: [`AgentEvent`], [`AgentStep`], [`AgentResponse`], [`AgentStopReason`].

pub use agent::{Agent, AgentBuilder, AgentConfig};
pub use dispatch::Dispatcher;
pub use event::{AgentEvent, AgentResponse, AgentStep, AgentStopReason};

mod agent;
mod dispatch;
mod event;
pub mod model;
