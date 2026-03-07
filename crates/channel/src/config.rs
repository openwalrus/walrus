//! Channel configuration types.

use serde::{Deserialize, Serialize};

/// Top-level channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfig {
    /// Telegram bot configuration.
    pub telegram: Option<TelegramConfig>,
    /// Discord bot configuration.
    pub discord: Option<DiscordConfig>,
}

/// Configuration for the Telegram bot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot API token.
    pub bot: String,
    /// Agent to route messages to. Falls back to `default_agent` if absent.
    pub agent: Option<String>,
}

/// Configuration for the Discord bot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Bot token.
    pub token: String,
    /// Agent to route messages to. Falls back to `default_agent` if absent.
    pub agent: Option<String>,
}
