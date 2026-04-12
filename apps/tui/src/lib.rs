//! Crabtalk CLI — thin client for the crabtalk daemon. Connects via Unix domain socket.

pub use cmd::{Cli, Command};

pub mod cmd;
pub mod repl;
pub mod tui;
