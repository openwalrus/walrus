//! Tests for `ProviderManager`.

use walrus_provider::{ProviderConfig, ProviderKind, ProviderManager};

fn test_configs() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            name: "primary".into(),
            provider: ProviderKind::DeepSeek,
            model: "deepseek-chat".into(),
            api_key: "key1".to_string(),
            base_url: None,
        },
        ProviderConfig {
            name: "secondary".into(),
            provider: ProviderKind::OpenAI,
            model: "gpt-4o".into(),
            api_key: "key2".to_string(),
            base_url: None,
        },
    ]
}

#[test]
fn test_manager_add_and_switch() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).unwrap();

    // First config is active by default.
    assert_eq!(manager.active_name().as_str(), "primary");

    // Switch to secondary.
    manager.switch("secondary").unwrap();
    assert_eq!(manager.active_name().as_str(), "secondary");

    // Add a third provider and switch to it.
    let third = ProviderConfig {
        name: "third".into(),
        provider: ProviderKind::Claude,
        model: "claude-sonnet-4-6".into(),
        api_key: "key3".to_string(),
        base_url: None,
    };
    manager.add(&third).unwrap();
    manager.switch("third").unwrap();
    assert_eq!(manager.active_name().as_str(), "third");
}

#[test]
fn test_manager_remove_active_fails() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).unwrap();

    let result = manager.remove("primary");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("cannot remove the active provider")
    );
}

#[test]
fn test_manager_remove_inactive() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).unwrap();

    manager.remove("secondary").unwrap();
    let entries = manager.list();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name.as_str(), "primary");
}

#[test]
fn test_manager_list_shows_active() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).unwrap();

    let entries = manager.list();
    assert_eq!(entries.len(), 2);

    let primary = entries.iter().find(|e| e.name == "primary").unwrap();
    let secondary = entries.iter().find(|e| e.name == "secondary").unwrap();
    assert!(primary.active);
    assert!(!secondary.active);
}

#[test]
fn test_manager_switch_unknown_fails() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).unwrap();

    let result = manager.switch("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_manager_remove_unknown_fails() {
    let configs = test_configs();
    let manager = ProviderManager::from_configs(&configs).unwrap();

    let result = manager.remove("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_manager_empty_configs_fails() {
    let result = ProviderManager::from_configs(&[]);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("at least one provider")
    );
}
