//! Tests for `build_provider()` factory (DD#67).

use walrus_model::{Provider, ProviderConfig, build_provider};

#[tokio::test]
async fn build_deepseek_default() {
    let config = ProviderConfig {
        model: "deepseek-chat".into(),
        api_key: Some("test-key".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    let p = build_provider(&config, walrus_model::Client::new()).await.unwrap();
    assert!(matches!(p, Provider::DeepSeek(_)));
}

#[tokio::test]
async fn build_openai_custom_url() {
    let config = ProviderConfig {
        model: "gpt-4o".into(),
        api_key: Some("test-key".into()),
        base_url: Some("http://localhost:8080/v1".into()),
        loader: None,
        quantization: None,
        chat_template: None,
    };
    let p = build_provider(&config, walrus_model::Client::new()).await.unwrap();
    assert!(matches!(p, Provider::OpenAI(_)));
}

#[tokio::test]
async fn build_ollama_base_url_only() {
    let config = ProviderConfig {
        model: "gpt-4o".into(),
        api_key: None,
        base_url: Some("http://localhost:11434/v1/chat/completions".into()),
        loader: None,
        quantization: None,
        chat_template: None,
    };
    let p = build_provider(&config, walrus_model::Client::new()).await.unwrap();
    assert!(matches!(p, Provider::OpenAI(_)));
}

#[tokio::test]
async fn build_claude_default() {
    let config = ProviderConfig {
        model: "claude-sonnet-4-6".into(),
        api_key: Some("test-key".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    let p = build_provider(&config, walrus_model::Client::new()).await.unwrap();
    assert!(matches!(p, Provider::Claude(_)));
}
