//! Tests for `ProviderConfig`.

use walrus_provider::ProviderConfig;

#[test]
fn test_provider_config_default_name() {
    let json = r#"{"model": "gpt-4o", "api_key": "key"}"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.name.as_str(), "default");
}

#[test]
fn test_provider_config_custom_name() {
    let json = r#"{"name": "prod", "model": "gpt-4o", "api_key": "key"}"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.name.as_str(), "prod");
}

#[test]
fn test_provider_config_roundtrip() {
    let json = r#"{"name":"test","provider":"mistral","model":"mistral-small","api_key":"k"}"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.name.as_str(), "test");
    assert_eq!(config.provider, walrus_provider::ProviderKind::Mistral);
    assert_eq!(config.model.as_str(), "mistral-small");
}
