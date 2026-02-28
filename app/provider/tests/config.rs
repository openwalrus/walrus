//! Tests for `ProviderConfig`.

use walrus_provider::{ProviderConfig, config::BackendConfig};

#[test]
fn test_provider_config_deepseek_from_json() {
    let json = r#"{"provider": "deep_seek", "model": "deepseek-chat", "api_key": "key"}"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.name.as_str(), "default");
    assert_eq!(config.model.as_str(), "deepseek-chat");
    assert!(matches!(config.backend, BackendConfig::DeepSeek(_)));
}

#[test]
fn test_provider_config_custom_name() {
    let json =
        r#"{"name": "prod", "provider": "openai", "model": "gpt-4o", "api_key": "key"}"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.name.as_str(), "prod");
    assert!(matches!(config.backend, BackendConfig::OpenAI(_)));
}

#[test]
fn test_provider_config_claude() {
    let json = r#"{"provider": "claude", "model": "claude-sonnet", "api_key": "k"}"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.kind(), "claude");
}

#[test]
fn test_provider_config_ollama() {
    let json = r#"{"provider": "ollama", "model": "llama3"}"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.kind(), "ollama");
}

#[cfg(feature = "local")]
#[test]
fn test_provider_config_local_hf() {
    let json = r#"{
        "provider": "local",
        "model": "phi-3.5-mini",
        "model_id": "microsoft/Phi-3.5-mini-instruct",
        "quantization": "q4k"
    }"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.kind(), "local");
    match &config.backend {
        BackendConfig::Local(lc) => {
            assert_eq!(lc.model_id.as_deref(), Some("microsoft/Phi-3.5-mini-instruct"));
            assert!(lc.quantization.is_some());
        }
        _ => panic!("expected Local backend"),
    }
}

#[cfg(feature = "local")]
#[test]
fn test_provider_config_local_gguf() {
    let json = r#"{
        "provider": "local",
        "model": "mistral-7b",
        "model_path": "/models/mistral/",
        "model_files": ["mistral-7b.Q4_K_M.gguf"]
    }"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.kind(), "local");
    match &config.backend {
        BackendConfig::Local(lc) => {
            assert_eq!(lc.model_path.as_deref(), Some("/models/mistral/"));
            assert_eq!(lc.model_files.len(), 1);
        }
        _ => panic!("expected Local backend"),
    }
}
