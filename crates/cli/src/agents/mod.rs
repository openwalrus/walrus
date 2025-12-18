//! CLI Agents

pub use anto::Anto;
use clap::ValueEnum;

mod anto;

/// Available agent types
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AgentKind {
    /// Anto - basic agent with echo tool for testing
    Anto,
}
