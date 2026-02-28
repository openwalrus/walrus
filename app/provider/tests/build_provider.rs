//! Tests for `build_provider()` factory.

use walrus_provider::{ProviderConfig, ProviderKind, Provider, build_provider};

#[test]
fn test_build_provider_deepseek_default() {
    let config = ProviderConfig {
        name: "ds".into(),
        provider: ProviderKind::DeepSeek,
        model: "deepseek-chat".into(),
        api_key: "test-key".to_string(),
        base_url: None,
    };
    let p = build_provider(&config, llm::Client::new()).unwrap();
    assert!(matches!(p, Provider::DeepSeek(_)));
}

#[test]
fn test_build_provider_openai_custom_url() {
    let config = ProviderConfig {
        name: "oai".into(),
        provider: ProviderKind::OpenAI,
        model: "gpt-4o".into(),
        api_key: "test-key".to_string(),
        base_url: Some("http://localhost:8080/v1".to_string()),
    };
    let p = build_provider(&config, llm::Client::new()).unwrap();
    assert!(matches!(p, Provider::OpenAI(_)));
}

#[test]
fn test_build_provider_ollama_no_key() {
    let config = ProviderConfig {
        name: "ollama".into(),
        provider: ProviderKind::Ollama,
        model: "llama3".into(),
        api_key: String::new(),
        base_url: None,
    };
    let p = build_provider(&config, llm::Client::new()).unwrap();
    assert!(matches!(p, Provider::OpenAI(_)));
}

#[test]
fn test_build_provider_claude_default() {
    let config = ProviderConfig {
        name: "claude".into(),
        provider: ProviderKind::Claude,
        model: "claude-sonnet-4-6".into(),
        api_key: "test-key".to_string(),
        base_url: None,
    };
    let p = build_provider(&config, llm::Client::new()).unwrap();
    assert!(matches!(p, Provider::Claude(_)));
}

#[test]
fn test_build_provider_mistral_default() {
    let config = ProviderConfig {
        name: "mistral".into(),
        provider: ProviderKind::Mistral,
        model: "mistral-small-latest".into(),
        api_key: "test-key".to_string(),
        base_url: None,
    };
    let p = build_provider(&config, llm::Client::new()).unwrap();
    assert!(matches!(p, Provider::Mistral(_)));
}

#[test]
fn test_build_provider_mistral_custom_endpoint() {
    let config = ProviderConfig {
        name: "mistral-local".into(),
        provider: ProviderKind::Mistral,
        model: "mistral-small-latest".into(),
        api_key: "test-key".to_string(),
        base_url: Some("http://localhost:8080/v1/chat/completions".to_string()),
    };
    let p = build_provider(&config, llm::Client::new()).unwrap();
    assert!(matches!(p, Provider::Mistral(_)));
}
