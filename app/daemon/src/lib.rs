//! Walrus gateway — application shell composing runtime, channels, and cron
//! scheduling. Personal agent, local-first.

pub mod channel;
pub mod config;
mod feature;
pub mod gateway;
pub mod utils;

pub use channel::router::{ChannelRouter, RoutingRule};
pub use config::GatewayConfig;
pub use feature::{
    cron::{CronJob, CronScheduler},
    memory::MemoryBackend,
};
pub use gateway::{
    Gateway, GatewayHook,
    builder::build_runtime,
    serve::{ServeHandle, serve, serve_with_config},
};
