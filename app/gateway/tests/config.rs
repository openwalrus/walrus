//! Gateway configuration tests.

use walrus_gateway::{
    GatewayConfig,
    config::{MemoryBackendKind, ProviderKind},
};

#[test]
fn parse_minimal_config() {
    let toml = r#"
[server]

[llm]
model = "deepseek-chat"
api_key = "test-key"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    assert!(config.server.socket_path.is_none());
    assert_eq!(config.llm.model.as_str(), "deepseek-chat");
    assert_eq!(config.llm.api_key, "test-key");
    assert!(config.channels.is_empty());
}

#[test]
fn parse_full_config() {
    let toml = r#"
[server]
socket_path = "/tmp/walrus.sock"

[llm]
model = "deepseek-chat"
api_key = "sk-test"

[memory]
backend = "sqlite"

[[channels]]
platform = "telegram"
bot_token = "bot-token-123"
agent = "assistant"

[[mcp_servers]]
name = "playwright"
command = "npx"
args = ["playwright-mcp"]
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    assert_eq!(
        config.server.socket_path.as_deref(),
        Some("/tmp/walrus.sock")
    );
    assert_eq!(config.memory.backend, MemoryBackendKind::Sqlite);
    assert_eq!(config.channels.len(), 1);
    assert_eq!(config.mcp_servers.len(), 1);
    assert_eq!(config.mcp_servers[0].name.as_str(), "playwright");
    assert!(config.mcp_servers[0].auto_restart);
}

#[test]
fn default_server_config() {
    let toml = r#"
[server]

[llm]
model = "deepseek-chat"
api_key = "key"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    assert!(config.server.socket_path.is_none());
    assert_eq!(config.llm.provider, ProviderKind::DeepSeek);
}

#[test]
fn default_memory_config() {
    let toml = r#"
[server]

[llm]
model = "deepseek-chat"
api_key = "key"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    assert_eq!(config.memory.backend, MemoryBackendKind::InMemory);
}

#[test]
fn default_socket_path() {
    let toml = r#"
[server]

[llm]
model = "deepseek-chat"
api_key = "key"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    let path = config.socket_path(std::path::Path::new("/tmp/walrus"));
    assert_eq!(path, std::path::PathBuf::from("/tmp/walrus/walrus.sock"));
}

#[test]
fn custom_socket_path() {
    let toml = r#"
[server]
socket_path = "/run/walrus/custom.sock"

[llm]
model = "deepseek-chat"
api_key = "key"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    let path = config.socket_path(std::path::Path::new("/tmp/walrus"));
    assert_eq!(path, std::path::PathBuf::from("/run/walrus/custom.sock"));
}

#[test]
fn env_var_expansion() {
    unsafe { std::env::set_var("TEST_WALRUS_KEY", "expanded-value") };
    let toml = r#"
[server]

[llm]
model = "deepseek-chat"
api_key = "${TEST_WALRUS_KEY}"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    assert_eq!(config.llm.api_key, "expanded-value");
    unsafe { std::env::remove_var("TEST_WALRUS_KEY") };
}

#[test]
fn mcp_server_config() {
    let toml = r#"
[server]

[llm]
model = "deepseek-chat"
api_key = "key"

[[mcp_servers]]
name = "test-server"
command = "test-cmd"
args = ["--flag"]
auto_restart = false

[mcp_servers.env]
KEY = "value"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    assert_eq!(config.mcp_servers.len(), 1);
    let mcp = &config.mcp_servers[0];
    assert_eq!(mcp.name.as_str(), "test-server");
    assert_eq!(mcp.command, "test-cmd");
    assert_eq!(mcp.args, vec!["--flag"]);
    assert!(!mcp.auto_restart);
    assert_eq!(mcp.env.get("KEY").unwrap(), "value");
}

#[test]
fn global_config_dir_is_under_platform_config() {
    let dir = walrus_gateway::config::global_config_dir();
    // Should end with "walrus"
    assert_eq!(dir.file_name().unwrap(), "walrus");
}

#[test]
fn parse_mistral_provider() {
    let toml = r#"
[server]

[llm]
provider = "mistral"
model = "mistral-small-latest"
api_key = "test-key"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    assert_eq!(config.llm.provider, ProviderKind::Mistral);
    assert_eq!(config.llm.model.as_str(), "mistral-small-latest");
}

#[test]
fn parse_mistral_with_base_url() {
    let toml = r#"
[server]

[llm]
provider = "mistral"
model = "mistral-small-latest"
api_key = "test-key"
base_url = "http://localhost:8080/v1/chat/completions"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    assert_eq!(config.llm.provider, ProviderKind::Mistral);
    assert_eq!(
        config.llm.base_url.as_deref(),
        Some("http://localhost:8080/v1/chat/completions")
    );
}

#[test]
fn provider_kind_mistral_roundtrip() {
    let kind = ProviderKind::Mistral;
    let serialized = serde_json::to_string(&kind).unwrap();
    assert_eq!(serialized, "\"mistral\"");
    let parsed: ProviderKind = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed, ProviderKind::Mistral);
}
