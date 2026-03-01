//! Tests for `Provider::context_length()` (DD#68, P18-04).

use walrus_model::{Provider, ProviderConfig, build_provider};

#[tokio::test]
async fn provider_remote_context_length_none() {
    let client = walrus_model::Client::new();

    // DeepSeek returns None
    let config = ProviderConfig {
        model: "deepseek-chat".into(),
        api_key: Some("test-key".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    let p = build_provider(&config, client.clone()).await.unwrap();
    assert!(matches!(p, Provider::DeepSeek(_)));
    assert_eq!(p.context_length("deepseek-chat"), None);

    // OpenAI returns None
    let config = ProviderConfig {
        model: "gpt-4o".into(),
        api_key: Some("test-key".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    let p = build_provider(&config, client.clone()).await.unwrap();
    assert!(matches!(p, Provider::OpenAI(_)));
    assert_eq!(p.context_length("gpt-4o"), None);

    // Claude returns None
    let config = ProviderConfig {
        model: "claude-sonnet-4-6".into(),
        api_key: Some("test-key".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    let p = build_provider(&config, client).await.unwrap();
    assert!(matches!(p, Provider::Claude(_)));
    assert_eq!(p.context_length("claude-sonnet-4-6"), None);
}
