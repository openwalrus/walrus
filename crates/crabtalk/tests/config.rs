use crabtalk::{DaemonConfig, storage::DEFAULT_CONFIG};

#[test]
fn parse_default_config_template() {
    DaemonConfig::from_toml(DEFAULT_CONFIG).expect("default config template should parse");
}

#[test]
fn empty_config() {
    let config = DaemonConfig::from_toml("").unwrap();
    assert!(config.provider.is_empty());
    assert!(config.env.is_empty());
}

#[test]
fn invalid_toml_syntax() {
    let result = DaemonConfig::from_toml("this is not [valid toml");
    assert!(result.is_err());
}

#[test]
fn task_defaults() {
    let config = DaemonConfig::from_toml("").unwrap();
    assert_eq!(config.tasks.max_concurrent, 4);
    assert_eq!(config.tasks.viewable_window, 16);
    assert_eq!(config.tasks.task_timeout, 300);
}

#[test]
fn task_overrides() {
    let toml = r#"
[tasks]
max_concurrent = 8
task_timeout = 600
"#;
    let config = DaemonConfig::from_toml(toml).unwrap();
    assert_eq!(config.tasks.max_concurrent, 8);
    assert_eq!(config.tasks.task_timeout, 600);
}

#[test]
fn env_vars_parsed() {
    let toml = r#"
[env]
FOO = "bar"
BAZ = "qux"
"#;
    let config = DaemonConfig::from_toml(toml).unwrap();
    assert_eq!(config.env.get("FOO").unwrap(), "bar");
    assert_eq!(config.env.get("BAZ").unwrap(), "qux");
}

#[test]
fn provider_section_parsed() {
    let toml = r#"
[provider.openai]
api_key = "test-key"
models = ["gpt-4o"]
"#;
    let config = DaemonConfig::from_toml(toml).unwrap();
    let p = &config.provider["openai"];
    assert_eq!(p.api_key.as_deref(), Some("test-key"));
    assert_eq!(p.models, vec!["gpt-4o"]);
}

/// `[mcps.*]` and `[agents.*]` are mutable runtime records and live in
/// `local/settings.toml`, not config.toml. The parser ignores any
/// stray sections (TOML allows unknown keys via `#[serde(default)]`),
/// so a hand-edit doesn't crash the daemon — but the values are
/// silently dropped, which is the intended behavior.
#[test]
fn unknown_sections_ignored() {
    let toml = r#"
[mcps.myserver]
command = "my-mcp-server"

[agents.helper]
description = "A helper agent"
"#;
    DaemonConfig::from_toml(toml).unwrap();
}
