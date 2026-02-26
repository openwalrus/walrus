//! Walrus gateway — application shell composing runtime, channels,
//! sessions, authentication, and cron scheduling.

pub mod backend;
pub mod builder;
pub mod channel;
pub mod config;
pub mod cron;
pub mod hook;
pub mod session;
pub mod state;
pub mod utils;
pub mod ws;

pub use backend::MemoryBackend;
pub use builder::build_runtime;
pub use channel::{
    key::ApiKeyAuthenticator,
    auth::{AuthContext, AuthError, Authenticator},
    router::{ChannelRouter, RoutingRule},
};
pub use config::GatewayConfig;
pub use cron::{CronJob, CronScheduler};
pub use hook::GatewayHook;
pub use session::{Session, SessionManager, SessionScope, TrustLevel};
pub use state::AppState;
