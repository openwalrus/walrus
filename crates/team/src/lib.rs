//! Multi-agent interaction via the agent-as-tool pattern.
//!
//! This crate provides [`WithTeam<A, S>`], a tower-inspired layer that
//! enables an agent to delegate work to a sub-agent. The sub-agent runs
//! its own independent LLM conversation (via [`Chat::send()`]) and
//! returns the final text response as the tool result.
//!
//! Each layer adds exactly one sub-agent. Multiple sub-agents compose
//! by nesting — all types are monomorphized at compile time with zero
//! dynamic dispatch overhead:
//!
//! ```text
//! WithTeam<WithTeam<PerpAgent, Analyst>, Risk>
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use cydonia_team::{AgentSub, WithTeam};
//!
//! let analyst = AgentSub::new(
//!     "analyst",
//!     "Technical market analysis",
//!     provider.clone(),
//!     config.clone(),
//!     AnalystAgent::new(),
//! );
//!
//! let agent = PerpAgent::new(pool, &req);
//! let agent = WithTeam::new(agent, analyst);
//! let chat = Chat::with_tools(config, provider, agent, messages);
//! ```

pub use agent::WithTeam;
pub use layer::TeamLayer;
pub use sub::{AgentSub, SubAgent};

mod agent;
pub(crate) mod layer;
mod sub;
