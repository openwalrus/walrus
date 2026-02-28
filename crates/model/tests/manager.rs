//! Tests for `ProviderManager` (DD#67).

use walrus_model::{ProviderConfig, ProviderManager};

fn test_configs() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            model: "deepseek-chat".into(),
            api_key: Some("key1".into()),
            base_url: None,
            loader: None,
            quantization: None,
            chat_template: None,
        },
        ProviderConfig {
            model: "gpt-4o".into(),
            api_key: Some("key2".into()),
            base_url: None,
            loader: None,
            quantization: None,
            chat_template: None,
        },
    ]
}

#[tokio::test]
async fn first_config_is_active() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    assert_eq!(manager.active_model().as_str(), "deepseek-chat");
}

#[tokio::test]
async fn switch_and_active_config() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    manager.switch("gpt-4o").unwrap();
    assert_eq!(manager.active_model().as_str(), "gpt-4o");
    let config = manager.active_config();
    assert_eq!(config.model.as_str(), "gpt-4o");
}

#[tokio::test]
async fn add_and_switch() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    let third = ProviderConfig {
        model: "claude-sonnet-4-6".into(),
        api_key: Some("key3".into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    manager.add(&third).await.unwrap();
    manager.switch("claude-sonnet-4-6").unwrap();
    assert_eq!(manager.active_model().as_str(), "claude-sonnet-4-6");
}

#[tokio::test]
async fn add_validates_config() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    // Missing api_key and base_url for remote model — should fail validation.
    let invalid = ProviderConfig {
        model: "deepseek-chat".into(),
        api_key: None,
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    assert!(manager.add(&invalid).await.is_err());
}

#[tokio::test]
async fn remove_active_fails() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    let result = manager.remove("deepseek-chat");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("cannot remove the active provider")
    );
}

#[tokio::test]
async fn remove_inactive() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    manager.remove("gpt-4o").unwrap();
    let entries = manager.list();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name.as_str(), "deepseek-chat");
}

#[tokio::test]
async fn list_shows_active() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    let entries = manager.list();
    assert_eq!(entries.len(), 2);
    let ds = entries.iter().find(|e| e.name == "deepseek-chat").unwrap();
    let oai = entries.iter().find(|e| e.name == "gpt-4o").unwrap();
    assert!(ds.active);
    assert!(!oai.active);
}

#[tokio::test]
async fn switch_unknown_fails() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    let result = manager.switch("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn remove_unknown_fails() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    let result = manager.remove("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn empty_configs_fails() {
    let configs: Vec<ProviderConfig> = Vec::new();
    let result = ProviderManager::from_configs(&configs).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("at least one provider")
    );
}
