//! Channel configuration types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported channel platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    /// Telegram bot via long-polling.
    Telegram,
    /// Discord bot via WebSocket gateway.
    Discord,
}

impl ChannelType {
    /// All known variants, in definition order.
    pub const VARIANTS: &[Self] = &[Self::Telegram, Self::Discord];

    /// URL hint for obtaining a bot token for this platform.
    pub fn token_hint(self) -> &'static str {
        match self {
            Self::Telegram => "https://core.telegram.org/bots#botfather",
            Self::Discord => "https://discord.com/developers/applications",
        }
    }
}

impl fmt::Display for ChannelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Telegram => f.write_str("Telegram"),
            Self::Discord => f.write_str("Discord"),
        }
    }
}

/// A single channel entry — one bot connection to one agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEntry {
    /// Platform type.
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    /// Bot/API token.
    pub token: String,
    /// Agent to route messages to. Falls back to `default_agent` if absent.
    pub agent: Option<String>,
}

/// Top-level channel configuration — a list of channel entries.
///
/// Deserialized from `[[channel]]` TOML array of tables.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct ChannelConfig(pub Vec<ChannelEntry>);
