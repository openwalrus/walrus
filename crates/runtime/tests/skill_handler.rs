//! Tests for SkillHandler — skill loading through the Storage trait.

use crabtalk_runtime::{MemStorage, SkillHandler, SkillRoot, Storage};
use std::{path::PathBuf, sync::Arc};

fn write_skill(storage: &dyn Storage, name: &str) {
    let key = format!("{name}/SKILL.md");
    let content =
        format!("---\nname: {name}\ndescription: test skill\n---\nSkill body for {name}.");
    storage.put(&key, content.as_bytes()).unwrap();
}

fn root(storage: Arc<dyn Storage>, label: &str) -> SkillRoot {
    SkillRoot {
        label: PathBuf::from(label),
        storage,
    }
}

#[test]
fn load_from_single_dir() {
    let storage: Arc<dyn Storage> = Arc::new(MemStorage::new());
    write_skill(storage.as_ref(), "greet");
    write_skill(storage.as_ref(), "search");

    let handler = SkillHandler::load(vec![root(storage, "dir")], &[]).unwrap();
    let reg = handler.registry.blocking_lock();
    assert_eq!(reg.skills.len(), 2);
    assert!(reg.contains("greet"));
    assert!(reg.contains("search"));
}

#[test]
fn load_from_multiple_dirs() {
    let s1: Arc<dyn Storage> = Arc::new(MemStorage::new());
    let s2: Arc<dyn Storage> = Arc::new(MemStorage::new());
    write_skill(s1.as_ref(), "skill-a");
    write_skill(s2.as_ref(), "skill-b");

    let handler = SkillHandler::load(vec![root(s1, "dir1"), root(s2, "dir2")], &[]).unwrap();
    let reg = handler.registry.blocking_lock();
    assert_eq!(reg.skills.len(), 2);
}

#[test]
fn load_skips_missing_dir() {
    let empty: Arc<dyn Storage> = Arc::new(MemStorage::new());
    let storage: Arc<dyn Storage> = Arc::new(MemStorage::new());
    write_skill(storage.as_ref(), "exists");

    let handler =
        SkillHandler::load(vec![root(empty, "empty"), root(storage, "dir")], &[]).unwrap();
    let reg = handler.registry.blocking_lock();
    assert_eq!(reg.skills.len(), 1);
    assert!(reg.contains("exists"));
}

#[test]
fn load_empty_dir() {
    let storage: Arc<dyn Storage> = Arc::new(MemStorage::new());
    let handler = SkillHandler::load(vec![root(storage, "dir")], &[]).unwrap();
    let reg = handler.registry.blocking_lock();
    assert!(reg.skills.is_empty());
}

#[test]
fn load_conflict_first_dir_wins() {
    let s1: Arc<dyn Storage> = Arc::new(MemStorage::new());
    let s2: Arc<dyn Storage> = Arc::new(MemStorage::new());

    s1.put(
        "shared/SKILL.md",
        b"---\nname: shared\ndescription: from dir1\n---\nfirst body",
    )
    .unwrap();
    s2.put(
        "shared/SKILL.md",
        b"---\nname: shared\ndescription: from dir2\n---\nsecond body",
    )
    .unwrap();

    let handler = SkillHandler::load(vec![root(s1, "dir1"), root(s2, "dir2")], &[]).unwrap();
    let reg = handler.registry.blocking_lock();
    assert_eq!(reg.skills.len(), 1);
    // First dir wins — verify it's from dir1
    assert_eq!(reg.skills[0].description, "from dir1");
}

#[test]
fn load_skips_hidden_dirs() {
    let storage: Arc<dyn Storage> = Arc::new(MemStorage::new());
    write_skill(storage.as_ref(), "visible");
    storage
        .put(
            ".hidden/SKILL.md",
            b"---\nname: hidden\ndescription: x\n---\nbody",
        )
        .unwrap();

    let handler = SkillHandler::load(vec![root(storage, "dir")], &[]).unwrap();
    let reg = handler.registry.blocking_lock();
    assert_eq!(reg.skills.len(), 1);
    assert!(reg.contains("visible"));
    assert!(!reg.contains("hidden"));
}

#[test]
fn load_no_dirs() {
    let handler = SkillHandler::load(vec![], &[]).unwrap();
    let reg = handler.registry.blocking_lock();
    assert!(reg.skills.is_empty());
}
