//! Gateway configuration loaded from TOML.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Top-level gateway configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Server bind configuration.
    pub server: ServerConfig,
    /// LLM provider configuration.
    pub llm: LlmConfig,
    /// Memory backend configuration.
    #[serde(default)]
    pub memory: MemoryConfig,
    /// Agent definitions.
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
    /// Authentication configuration.
    #[serde(default)]
    pub auth: AuthConfig,
    /// Channel configurations.
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
    /// Cron job configurations.
    #[serde(default)]
    pub cron: Vec<CronConfig>,
    /// Skills directory configuration.
    pub skills: Option<SkillsConfig>,
    /// MCP server configurations.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}

/// Server bind configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host address to bind to.
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to listen on.
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 3000,
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

/// LLM provider configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Model identifier.
    pub model: CompactString,
    /// API key (supports `${ENV_VAR}` expansion).
    pub api_key: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "deepseek-chat".into(),
            api_key: "${DEEPSEEK_API_KEY}".to_owned(),
        }
    }
}

/// Memory backend configuration.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    /// Backend type: "in_memory" or "sqlite".
    pub backend: MemoryBackendKind,
    /// Database file path (sqlite only).
    pub path: Option<String>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: MemoryBackendKind::InMemory,
            path: None,
        }
    }
}

/// Memory backend kind.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryBackendKind {
    /// In-memory backend (no persistence).
    #[default]
    InMemory,
    /// SQLite-backed persistent memory.
    Sqlite,
}

/// Agent configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent name.
    pub name: CompactString,
    /// Agent description.
    #[serde(default)]
    pub description: CompactString,
    /// System prompt.
    #[serde(default)]
    pub system_prompt: String,
    /// Tool names this agent can use.
    #[serde(default)]
    pub tools: SmallVec<[CompactString; 8]>,
    /// Skill tags for matching skills.
    #[serde(default)]
    pub skill_tags: SmallVec<[CompactString; 4]>,
}

/// Authentication configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// API keys that grant access.
    pub api_keys: Vec<String>,
}

/// Channel configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Platform name.
    pub platform: CompactString,
    /// Bot token (supports `${ENV_VAR}` expansion).
    pub bot_token: String,
    /// Default agent for this channel.
    pub agent: CompactString,
    /// Optional specific channel ID for exact routing.
    pub channel_id: Option<CompactString>,
}

/// Cron job configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct CronConfig {
    /// Job name.
    pub name: CompactString,
    /// Cron schedule expression.
    pub schedule: String,
    /// Target agent name.
    pub agent: CompactString,
    /// Message template to send.
    pub message: String,
}

/// Skills directory configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct SkillsConfig {
    /// Path to the skills directory.
    pub directory: String,
}

/// MCP server configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name.
    pub name: CompactString,
    /// Command to spawn.
    pub command: String,
    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables.
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    /// Auto-restart on failure.
    #[serde(default = "default_true")]
    pub auto_restart: bool,
}

fn default_true() -> bool {
    true
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            llm: LlmConfig::default(),
            memory: MemoryConfig::default(),
            agents: vec![AgentConfig {
                name: "assistant".into(),
                description: "A helpful assistant".into(),
                system_prompt: "You are a helpful assistant. Be concise.".to_owned(),
                tools: SmallVec::new(),
                skill_tags: SmallVec::new(),
            }],
            auth: AuthConfig::default(),
            channels: Vec::new(),
            cron: Vec::new(),
            skills: None,
            mcp_servers: Vec::new(),
        }
    }
}

impl GatewayConfig {
    /// Parse a TOML string into a `GatewayConfig`, expanding environment
    /// variables in supported fields.
    pub fn from_toml(toml_str: &str) -> anyhow::Result<Self> {
        let expanded = crate::utils::expand_env_vars(toml_str);
        let config: Self = toml::from_str(&expanded)?;
        Ok(config)
    }

    /// Load configuration from a file path.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content)
    }

    /// Get the bind address as "host:port".
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}
