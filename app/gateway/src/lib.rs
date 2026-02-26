//! Walrus gateway — application shell composing runtime, channels,
//! sessions, authentication, and cron scheduling.

pub mod channel;
pub mod config;
mod feature;
pub mod gateway;
pub mod utils;

pub use channel::{
    auth::{AuthContext, AuthError, Authenticator},
    key::ApiKeyAuthenticator,
    router::{ChannelRouter, RoutingRule},
};
pub use config::GatewayConfig;
pub use feature::{
    cron::{CronJob, CronScheduler},
    memory::MemoryBackend,
};
pub use gateway::{
    Gateway, GatewayHook,
    builder::build_runtime,
    serve::{ServeHandle, serve, serve_with_config},
    session::SessionManager,
};
