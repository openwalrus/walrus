//! Tests for CLI preferences (TOML roundtrip).

use walrus_cli::prefs::CliPrefs;

#[test]
fn config_toml_roundtrip() {
    let prefs = CliPrefs {
        default_gateway: Some("ws://example.com/ws".to_owned()),
        default_agent: Some("helper".to_owned()),
        model: Some("deepseek-chat".to_owned()),
    };

    let toml_str = toml::to_string_pretty(&prefs).unwrap();
    let parsed: CliPrefs = toml::from_str(&toml_str).unwrap();

    assert_eq!(
        parsed.default_gateway.as_deref(),
        Some("ws://example.com/ws")
    );
    assert_eq!(parsed.default_agent.as_deref(), Some("helper"));
    assert_eq!(parsed.model.as_deref(), Some("deepseek-chat"));
}

#[test]
fn config_defaults_are_none() {
    let prefs = CliPrefs::default();
    assert!(prefs.default_gateway.is_none());
    assert!(prefs.default_agent.is_none());
    assert!(prefs.model.is_none());
}
