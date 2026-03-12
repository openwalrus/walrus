//! Walrus daemon — message central composing runtime, channels, and cron
//! scheduling. Personal agent, local-first.

pub mod config;
pub mod daemon;
pub mod ext;
pub mod hook;

pub use config::DaemonConfig;
pub use daemon::event::{DaemonEvent, DaemonEventSender};
#[cfg(feature = "tcp")]
pub use daemon::setup_tcp;
pub use daemon::{Daemon, DaemonHandle, bridge_shutdown, setup_channels, setup_socket};
pub use hook::DaemonHook;
