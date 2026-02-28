//! Tests for `ProviderManager` (DD#67).

use compact_str::CompactString;
use std::collections::BTreeMap;
use walrus_provider::{ProviderConfig, ProviderManager};

fn test_configs() -> BTreeMap<CompactString, ProviderConfig> {
    let mut map = BTreeMap::new();
    map.insert(
        "primary".into(),
        ProviderConfig {
            model: "deepseek-chat".into(),
            api_key: Some("key1".into()),
            base_url: None,
            loader: None,
            quantization: None,
            chat_template: None,
        },
    );
    map.insert(
        "secondary".into(),
        ProviderConfig {
            model: "gpt-4o".into(),
            api_key: Some("key2".into()),
            base_url: None,
            loader: None,
            quantization: None,
            chat_template: None,
        },
    );
    map
}

#[tokio::test]
async fn from_btreemap_first_key_active() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    // BTreeMap sorts alphabetically — "primary" < "secondary".
    assert_eq!(manager.active_name().as_str(), "primary");
}

#[tokio::test]
async fn active_model_returns_model() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    assert_eq!(manager.active_model().as_str(), "deepseek-chat");
}

#[tokio::test]
async fn switch_and_active_config() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    manager.switch("secondary").unwrap();
    assert_eq!(manager.active_name().as_str(), "secondary");
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
    manager.add("third", &third).await.unwrap();
    manager.switch("third").unwrap();
    assert_eq!(manager.active_name().as_str(), "third");
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
    assert!(manager.add("invalid", &invalid).await.is_err());
}

#[tokio::test]
async fn remove_active_fails() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    let result = manager.remove("primary");
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
    manager.remove("secondary").unwrap();
    let entries = manager.list();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name.as_str(), "primary");
}

#[tokio::test]
async fn list_shows_active() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();
    let entries = manager.list();
    assert_eq!(entries.len(), 2);
    let primary = entries.iter().find(|e| e.name == "primary").unwrap();
    let secondary = entries.iter().find(|e| e.name == "secondary").unwrap();
    assert!(primary.active);
    assert!(!secondary.active);
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
    let configs: BTreeMap<CompactString, ProviderConfig> = BTreeMap::new();
    let result = ProviderManager::from_configs(&configs).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("at least one provider")
    );
}
