//! Tests for `ProviderManager`.

use walrus_provider::{
    ProviderConfig, ProviderManager,
    config::{BackendConfig, RemoteConfig},
};

fn test_configs() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            name: "primary".into(),
            model: "deepseek-chat".into(),
            backend: BackendConfig::DeepSeek(RemoteConfig {
                api_key: "key1".to_string(),
                base_url: None,
            }),
        },
        ProviderConfig {
            name: "secondary".into(),
            model: "gpt-4o".into(),
            backend: BackendConfig::OpenAI(RemoteConfig {
                api_key: "key2".to_string(),
                base_url: None,
            }),
        },
    ]
}

#[tokio::test]
async fn test_manager_add_and_switch() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();

    // First config is active by default.
    assert_eq!(manager.active_name().as_str(), "primary");

    // Switch to secondary.
    manager.switch("secondary").unwrap();
    assert_eq!(manager.active_name().as_str(), "secondary");

    // Add a third provider and switch to it.
    let third = ProviderConfig {
        name: "third".into(),
        model: "claude-sonnet-4-6".into(),
        backend: BackendConfig::Claude(RemoteConfig {
            api_key: "key3".to_string(),
            base_url: None,
        }),
    };
    manager.add(&third).await.unwrap();
    manager.switch("third").unwrap();
    assert_eq!(manager.active_name().as_str(), "third");
}

#[tokio::test]
async fn test_manager_remove_active_fails() {
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
async fn test_manager_remove_inactive() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();

    manager.remove("secondary").unwrap();
    let entries = manager.list();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name.as_str(), "primary");
}

#[tokio::test]
async fn test_manager_list_shows_active() {
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
async fn test_manager_switch_unknown_fails() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();

    let result = manager.switch("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_manager_remove_unknown_fails() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).await.unwrap();

    let result = manager.remove("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_manager_empty_configs_fails() {
    let result = ProviderManager::from_configs(&[]).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("at least one provider")
    );
}
