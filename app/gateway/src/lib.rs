//! Walrus gateway — application shell composing runtime, channels,
//! sessions, authentication, and cron scheduling.

pub mod builder;
pub mod channel;
pub mod config;
mod feature;
pub mod hook;
pub mod session;
pub mod state;
pub mod utils;
pub mod ws;

pub use builder::build_runtime;
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
pub use hook::GatewayHook;
pub use session::{Session, SessionManager, SessionScope, TrustLevel};
pub use state::AppState;
