//! Crabtalk daemon — message central composing runtime, channels, and cron
//! scheduling. Personal agent, local-first.

pub mod config;
pub mod cron;
pub mod daemon;
pub mod hook;

pub use config::DaemonConfig;
#[cfg(unix)]
pub use daemon::setup_socket;
pub use daemon::{
    bridge_shutdown,
    event::{DaemonEvent, DaemonEventSender},
    setup_tcp, Daemon, DaemonHandle,
};
pub use hook::DaemonHook;
