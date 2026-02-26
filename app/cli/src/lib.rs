//! Walrus CLI application — command-line interface with direct and gateway
//! modes for interacting with walrus agents.

pub use cmd::{Cli, Command};

pub mod cmd;
pub mod config;
pub mod prefs;
pub mod repl;
pub mod runner;
