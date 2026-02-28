//! Tests for `build_provider()` factory.

use walrus_provider::{
    Provider, ProviderConfig, build_provider,
    config::{BackendConfig, OllamaConfig, RemoteConfig},
};

#[tokio::test]
async fn test_build_provider_deepseek_default() {
    let config = ProviderConfig {
        name: "ds".into(),
        model: "deepseek-chat".into(),
        backend: BackendConfig::DeepSeek(RemoteConfig {
            api_key: "test-key".to_string(),
            base_url: None,
        }),
    };
    let p = build_provider(&config, llm::Client::new()).await.unwrap();
    assert!(matches!(p, Provider::DeepSeek(_)));
}

#[tokio::test]
async fn test_build_provider_openai_custom_url() {
    let config = ProviderConfig {
        name: "oai".into(),
        model: "gpt-4o".into(),
        backend: BackendConfig::OpenAI(RemoteConfig {
            api_key: "test-key".to_string(),
            base_url: Some("http://localhost:8080/v1".to_string()),
        }),
    };
    let p = build_provider(&config, llm::Client::new()).await.unwrap();
    assert!(matches!(p, Provider::OpenAI(_)));
}

#[tokio::test]
async fn test_build_provider_ollama_no_key() {
    let config = ProviderConfig {
        name: "ollama".into(),
        model: "llama3".into(),
        backend: BackendConfig::Ollama(OllamaConfig { base_url: None }),
    };
    let p = build_provider(&config, llm::Client::new()).await.unwrap();
    assert!(matches!(p, Provider::OpenAI(_)));
}

#[tokio::test]
async fn test_build_provider_claude_default() {
    let config = ProviderConfig {
        name: "claude".into(),
        model: "claude-sonnet-4-6".into(),
        backend: BackendConfig::Claude(RemoteConfig {
            api_key: "test-key".to_string(),
            base_url: None,
        }),
    };
    let p = build_provider(&config, llm::Client::new()).await.unwrap();
    assert!(matches!(p, Provider::Claude(_)));
}
