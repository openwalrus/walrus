//! MCP CRUD surface — Storage::{list,load,upsert,delete}_mcp on FsStorage.

use crabtalk::storage::FsStorage;
use std::collections::BTreeMap;
use wcore::{McpServerConfig, storage::Storage};

fn fs_storage(config_dir: std::path::PathBuf) -> FsStorage {
    let sessions = config_dir.join("sessions");
    FsStorage::new(config_dir, sessions, Vec::new())
}

fn mcp(name: &str, command: &str) -> McpServerConfig {
    McpServerConfig {
        name: name.to_owned(),
        command: command.to_owned(),
        args: vec!["--flag".into()],
        env: BTreeMap::from([("KEY".into(), "value".into())]),
        auto_restart: true,
        url: None,
        auth: false,
    }
}

#[test]
fn empty_storage_lists_no_mcps() {
    let dir = tempfile::tempdir().unwrap();
    let storage = fs_storage(dir.path().to_path_buf());
    assert!(storage.list_mcps().unwrap().is_empty());
    assert!(storage.load_mcp("nonexistent").unwrap().is_none());
}

#[test]
fn upsert_then_load_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let storage = fs_storage(dir.path().to_path_buf());

    let server = mcp("playwright", "npx");
    storage.upsert_mcp(&server).unwrap();

    let loaded = storage.load_mcp("playwright").unwrap().unwrap();
    assert_eq!(loaded.name, "playwright");
    assert_eq!(loaded.command, "npx");
    assert_eq!(loaded.args, vec!["--flag".to_string()]);
    assert_eq!(loaded.env.get("KEY").map(String::as_str), Some("value"));
}

#[test]
fn upsert_overwrites_existing_entry() {
    let dir = tempfile::tempdir().unwrap();
    let storage = fs_storage(dir.path().to_path_buf());

    storage.upsert_mcp(&mcp("srv", "v1")).unwrap();
    storage.upsert_mcp(&mcp("srv", "v2")).unwrap();
    assert_eq!(storage.load_mcp("srv").unwrap().unwrap().command, "v2");
}

#[test]
fn upsert_rejects_invalid_names() {
    let dir = tempfile::tempdir().unwrap();
    let storage = fs_storage(dir.path().to_path_buf());

    for bad in [
        "",
        "with.dot",
        "br[acket",
        "br]acket",
        "qu\"ote",
        "ctrl\nchar",
    ] {
        let cfg = mcp(bad, "cmd");
        assert!(
            storage.upsert_mcp(&cfg).is_err(),
            "upsert_mcp accepted bad name: {bad:?}"
        );
    }
}

#[test]
fn delete_returns_existed_flag() {
    let dir = tempfile::tempdir().unwrap();
    let storage = fs_storage(dir.path().to_path_buf());

    storage.upsert_mcp(&mcp("srv", "cmd")).unwrap();
    assert!(storage.delete_mcp("srv").unwrap());
    assert!(!storage.delete_mcp("srv").unwrap());
    assert!(storage.load_mcp("srv").unwrap().is_none());
}

#[test]
fn list_returns_all_mcps_sorted() {
    let dir = tempfile::tempdir().unwrap();
    let storage = fs_storage(dir.path().to_path_buf());

    storage.upsert_mcp(&mcp("zeta", "z")).unwrap();
    storage.upsert_mcp(&mcp("alpha", "a")).unwrap();

    let names: Vec<_> = storage.list_mcps().unwrap().into_keys().collect();
    assert_eq!(names, vec!["alpha".to_string(), "zeta".to_string()]);
}

#[test]
fn mcps_persist_across_storage_instances() {
    let dir = tempfile::tempdir().unwrap();
    {
        let storage = fs_storage(dir.path().to_path_buf());
        storage.upsert_mcp(&mcp("persisted", "cmd")).unwrap();
    }
    let storage = fs_storage(dir.path().to_path_buf());
    let loaded = storage.load_mcp("persisted").unwrap().unwrap();
    assert_eq!(loaded.command, "cmd");
}

#[test]
fn mcp_serialized_under_mcps_table() {
    let dir = tempfile::tempdir().unwrap();
    let storage = fs_storage(dir.path().to_path_buf());

    storage.upsert_mcp(&mcp("srv", "cmd")).unwrap();

    let body = std::fs::read_to_string(dir.path().join("local").join("settings.toml")).unwrap();
    assert!(body.contains("[mcps.srv]"));
}
