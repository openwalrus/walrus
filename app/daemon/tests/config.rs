//! Gateway configuration tests (DD#67).

use walrus_daemon::{GatewayConfig, config::MemoryBackendKind};

#[test]
fn parse_minimal_config() {
    let toml = r#"
[server]

[llm.default]
model = "deepseek-chat"
api_key = "test-key"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    assert!(config.server.socket_path.is_none());
    assert_eq!(config.models.len(), 1);
    let default = &config.models["default"];
    assert_eq!(default.model.as_str(), "deepseek-chat");
    assert!(config.channels.is_empty());
}

#[test]
fn parse_multi_provider() {
    let toml = r#"
[server]

[llm.ds]
model = "deepseek-chat"
api_key = "key1"

[llm.oai]
model = "gpt-4o"
api_key = "key2"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    assert_eq!(config.models.len(), 2);
    assert!(config.models.contains_key("ds"));
    assert!(config.models.contains_key("oai"));
    assert_eq!(config.models["ds"].model.as_str(), "deepseek-chat");
    assert_eq!(config.models["oai"].model.as_str(), "gpt-4o");
}

#[test]
fn parse_full_config() {
    let toml = r#"
[server]
socket_path = "/tmp/walrus.sock"

[llm.default]
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
fn default_memory_config() {
    let toml = r#"
[server]

[llm.default]
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

[llm.default]
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

[llm.default]
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

[llm.default]
model = "deepseek-chat"
api_key = "${TEST_WALRUS_KEY}"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    let default = &config.models["default"];
    assert_eq!(default.api_key.as_deref(), Some("expanded-value"));
    unsafe { std::env::remove_var("TEST_WALRUS_KEY") };
}

#[test]
fn mcp_server_config() {
    let toml = r#"
[server]

[llm.default]
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
    let dir = walrus_daemon::config::global_config_dir();
    assert_eq!(dir.file_name().unwrap(), "walrus");
}

#[test]
fn parse_claude_provider() {
    let toml = r#"
[server]

[llm.claude]
model = "claude-sonnet-4-20250514"
api_key = "test-key"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    let claude = &config.models["claude"];
    assert_eq!(claude.model.as_str(), "claude-sonnet-4-20250514");
    assert_eq!(claude.kind().unwrap().as_str(), "claude");
}

#[test]
fn parse_openai_with_base_url() {
    let toml = r#"
[server]

[llm.oai]
model = "gpt-4o"
api_key = "test-key"
base_url = "http://localhost:8080/v1/chat/completions"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    let oai = &config.models["oai"];
    assert_eq!(oai.kind().unwrap().as_str(), "openai");
    assert_eq!(
        oai.base_url.as_deref(),
        Some("http://localhost:8080/v1/chat/completions")
    );
}

#[test]
fn parse_local_provider() {
    let toml = r#"
[server]

[llm.phi]
model = "microsoft/Phi-3.5-mini-instruct"
quantization = "q4k"
"#;
    let config = GatewayConfig::from_toml(toml).unwrap();
    let phi = &config.models["phi"];
    assert_eq!(phi.kind().unwrap().as_str(), "local");
    assert_eq!(phi.model.as_str(), "microsoft/Phi-3.5-mini-instruct");
}

#[test]
fn default_config_serializes() {
    let config = GatewayConfig::default();
    let toml_str = toml::to_string_pretty(&config).unwrap();
    // Should roundtrip.
    let _parsed: GatewayConfig = toml::from_str(&toml_str).unwrap();
}
