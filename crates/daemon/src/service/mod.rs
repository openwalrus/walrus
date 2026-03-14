//! Managed child services — spawn, handshake, registry, shutdown.

pub mod config;
pub mod manager;

pub use config::ServiceConfig;
pub use manager::{ServiceHandle, ServiceManager, ServiceRegistry};
