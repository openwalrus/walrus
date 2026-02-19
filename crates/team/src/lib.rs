//! Pluggable agent teams.
//!
//! A [`Team<A>`] is a leader agent with a dynamic registry of workers.
//! Workers can be registered and removed at runtime. Each worker is
//! exposed as a tool the LLM can call.
//!
//! Communication between leader and workers goes through a [`Protocol`]
//! trait, allowing both in-process ([`Local`]) and remote transports.
//!
//! # Example
//!
//! ```rust,ignore
//! use cydonia_team::{Team, Worker};
//!
//! let mut team = Team::new(leader_agent);
//! team.register(Worker::local(analyst, provider.clone(), config.clone()));
//! let chat = Chat::new(config, provider, team, vec![]);
//! ```

pub use local::Local;
pub use protocol::Protocol;
pub use task::{Task, TaskResult};
pub use team::Team;
pub use worker::Worker;

mod local;
mod protocol;
mod task;
mod team;
mod worker;
