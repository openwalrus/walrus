//! Tests for `ProviderConfig` (DD#67).

use walrus_model::{ProviderConfig, ProviderKind};

// --- kind detection ---

#[test]
fn kind_deepseek() {
    let config = ProviderConfig {
        model: "deepseek-chat".into(),
        api_key: Some("k".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert_eq!(config.kind().unwrap(), ProviderKind::DeepSeek);
}

#[test]
fn kind_openai() {
    let config = ProviderConfig {
        model: "gpt-4o".into(),
        api_key: Some("k".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert_eq!(config.kind().unwrap(), ProviderKind::OpenAI);
}

#[test]
fn kind_openai_o_series() {
    for model in &["o1-preview", "o3-mini", "o4-mini"] {
        let config = ProviderConfig {
            model: (*model).into(),
            api_key: Some("k".into()),
            base_url: None,
            loader: None,
            quantization: None,
            chat_template: None,
        };
        assert_eq!(
            config.kind().unwrap(),
            ProviderKind::OpenAI,
            "model: {model}"
        );
    }
}

#[test]
fn kind_claude() {
    let config = ProviderConfig {
        model: "claude-sonnet-4-20250514".into(),
        api_key: Some("k".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert_eq!(config.kind().unwrap(), ProviderKind::Claude);
}

#[test]
fn kind_grok() {
    let config = ProviderConfig {
        model: "grok-3".into(),
        api_key: Some("k".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert_eq!(config.kind().unwrap(), ProviderKind::Grok);
}

#[test]
fn kind_qwen() {
    let config = ProviderConfig {
        model: "qwen-plus".into(),
        api_key: Some("k".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert_eq!(config.kind().unwrap(), ProviderKind::Qwen);
}

#[test]
fn kind_qwq() {
    let config = ProviderConfig {
        model: "qwq-32b".into(),
        api_key: Some("k".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert_eq!(config.kind().unwrap(), ProviderKind::Qwen);
}

#[test]
fn kind_kimi() {
    let config = ProviderConfig {
        model: "kimi-latest".into(),
        api_key: Some("k".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert_eq!(config.kind().unwrap(), ProviderKind::Kimi);
}

#[test]
fn kind_moonshot() {
    let config = ProviderConfig {
        model: "moonshot-v1-128k".into(),
        api_key: Some("k".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert_eq!(config.kind().unwrap(), ProviderKind::Kimi);
}

#[test]
fn kind_local() {
    let config = ProviderConfig {
        model: "microsoft/Phi-3.5-mini-instruct".into(),
        api_key: None,
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert_eq!(config.kind().unwrap(), ProviderKind::Local);
}

#[test]
fn kind_unknown_prefix_errors() {
    let config = ProviderConfig {
        model: "foobar-model".into(),
        api_key: Some("k".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert!(config.kind().is_err());
}

// --- validation ---

#[test]
fn validate_remote_ok() {
    let config = ProviderConfig {
        model: "deepseek-chat".into(),
        api_key: Some("key".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    config.validate().unwrap();
}

#[test]
fn validate_remote_with_base_url_no_key() {
    let config = ProviderConfig {
        model: "gpt-4o".into(),
        api_key: None,
        base_url: Some("http://localhost:8080/v1".into()),
        loader: None,
        quantization: None,
        chat_template: None,
    };
    config.validate().unwrap();
}

#[test]
fn validate_remote_missing_key_and_url() {
    let config = ProviderConfig {
        model: "deepseek-chat".into(),
        api_key: None,
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert!(config.validate().is_err());
}

#[test]
fn validate_local_rejects_api_key() {
    let config = ProviderConfig {
        model: "microsoft/Phi-3.5-mini-instruct".into(),
        api_key: Some("key".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("must not have api_key"));
}

#[test]
fn validate_remote_rejects_loader() {
    use walrus_model::config::Loader;
    let config = ProviderConfig {
        model: "deepseek-chat".into(),
        api_key: Some("key".into()),
        base_url: None,
        loader: Some(Loader::Text),
        quantization: None,
        chat_template: None,
    };
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("must not have loader"));
}

#[test]
fn validate_empty_model() {
    let config = ProviderConfig {
        model: "".into(),
        api_key: None,
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("model is required"));
}

// --- serde round-trip ---

#[test]
fn deserialize_flat_remote() {
    let json = r#"{"model": "deepseek-chat", "api_key": "test-key"}"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.model.as_str(), "deepseek-chat");
    assert_eq!(config.api_key.as_deref(), Some("test-key"));
    assert!(config.loader.is_none());
}

#[test]
fn deserialize_flat_local_with_loader() {
    let json =
        r#"{"model": "microsoft/Phi-3.5-mini-instruct", "loader": "gguf", "quantization": "q4k"}"#;
    let config: ProviderConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.kind().unwrap(), ProviderKind::Local);
    assert!(config.loader.is_some());
    assert!(config.quantization.is_some());
}
