//! Daemon configuration tests (DD#67).

use walrus_daemon::DaemonConfig;

#[test]
fn parse_minimal_config() {
    let toml = r#"
[[models]]
model = "deepseek-chat"
api_key = "test-key"
"#;
    let config = DaemonConfig::from_toml(toml).unwrap();
    assert_eq!(config.models.len(), 1);
    assert_eq!(config.models[0].model.as_str(), "deepseek-chat");
    assert!(config.channels.is_empty());
}

#[test]
fn parse_multi_provider() {
    let toml = r#"
[[models]]
model = "deepseek-chat"
api_key = "key1"

[[models]]
model = "gpt-4o"
api_key = "key2"
"#;
    let config = DaemonConfig::from_toml(toml).unwrap();
    assert_eq!(config.models.len(), 2);
    assert_eq!(config.models[0].model.as_str(), "deepseek-chat");
    assert_eq!(config.models[1].model.as_str(), "gpt-4o");
}

#[test]
fn parse_full_config() {
    let toml = r#"
[[models]]
model = "deepseek-chat"
api_key = "sk-test"

[[channels]]
platform = "telegram"
bot_token = "bot-token-123"
agent = "assistant"

[[mcp_servers]]
name = "playwright"
command = "npx"
args = ["playwright-mcp"]
"#;
    let config = DaemonConfig::from_toml(toml).unwrap();
    assert_eq!(config.channels.len(), 1);
    assert_eq!(config.mcp_servers.len(), 1);
    assert_eq!(config.mcp_servers[0].name.as_str(), "playwright");
    assert!(config.mcp_servers[0].auto_restart);
}

#[test]
fn env_var_expansion() {
    unsafe { std::env::set_var("TEST_WALRUS_KEY", "expanded-value") };
    let toml = r#"
[[models]]
model = "deepseek-chat"
api_key = "${TEST_WALRUS_KEY}"
"#;
    let config = DaemonConfig::from_toml(toml).unwrap();
    assert_eq!(config.models[0].api_key.as_deref(), Some("expanded-value"));
    unsafe { std::env::remove_var("TEST_WALRUS_KEY") };
}

#[test]
fn mcp_server_config() {
    let toml = r#"
[[models]]
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
    let config = DaemonConfig::from_toml(toml).unwrap();
    assert_eq!(config.mcp_servers.len(), 1);
    let mcp = &config.mcp_servers[0];
    assert_eq!(mcp.name.as_str(), "test-server");
    assert_eq!(mcp.command, "test-cmd");
    assert_eq!(mcp.args, vec!["--flag"]);
    assert!(!mcp.auto_restart);
    assert_eq!(mcp.env.get("KEY").unwrap(), "value");
}

#[test]
fn global_config_dir_is_under_home() {
    let dir = walrus_daemon::config::global_config_dir();
    assert_eq!(dir.file_name().unwrap(), ".walrus");
}

#[test]
fn socket_path_is_under_walrus_dir() {
    let path = walrus_daemon::config::socket_path();
    assert_eq!(path.file_name().unwrap(), "walrus.sock");
    assert_eq!(path.parent().unwrap().file_name().unwrap(), ".walrus");
}

#[test]
fn parse_claude_provider() {
    let toml = r#"
[[models]]
model = "claude-sonnet-4-20250514"
api_key = "test-key"
"#;
    let config = DaemonConfig::from_toml(toml).unwrap();
    assert_eq!(config.models[0].model.as_str(), "claude-sonnet-4-20250514");
    assert_eq!(config.models[0].kind().unwrap().as_str(), "claude");
}

#[test]
fn parse_openai_with_base_url() {
    let toml = r#"
[[models]]
model = "gpt-4o"
api_key = "test-key"
base_url = "http://localhost:8080/v1/chat/completions"
"#;
    let config = DaemonConfig::from_toml(toml).unwrap();
    assert_eq!(config.models[0].kind().unwrap().as_str(), "openai");
    assert_eq!(
        config.models[0].base_url.as_deref(),
        Some("http://localhost:8080/v1/chat/completions")
    );
}

#[test]
fn parse_local_provider() {
    let toml = r#"
[[models]]
model = "microsoft/Phi-3.5-mini-instruct"
quantization = "q4k"
"#;
    let config = DaemonConfig::from_toml(toml).unwrap();
    assert_eq!(config.models[0].kind().unwrap().as_str(), "local");
    assert_eq!(
        config.models[0].model.as_str(),
        "microsoft/Phi-3.5-mini-instruct"
    );
}

#[test]
fn default_config_serializes() {
    let config = DaemonConfig::default();
    let toml_str = toml::to_string_pretty(&config).unwrap();
    // Should roundtrip.
    let _parsed: DaemonConfig = toml::from_str(&toml_str).unwrap();
}
