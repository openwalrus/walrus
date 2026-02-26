//! Walrus CLI application — command-line interface for interacting with
//! walrus agents in direct mode.

pub use cmd::{Cli, Command};

pub mod cmd;
pub mod config;
pub mod repl;
pub mod runner;
