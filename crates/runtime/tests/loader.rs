//! Tests for the markdown-based config loader.

use walrus_runtime::{load_agents_dir, load_cron_dir, parse_agent_md, parse_cron_md};

#[test]
fn parse_agent_md_roundtrip() {
    let md = r#"---
name: helper
description: A test agent
tools:
  - remember
  - search
skill_tags:
  - coding
---

You are a helpful coding assistant.
Be concise and clear.
"#;
    let agent = parse_agent_md(md).unwrap();
    assert_eq!(agent.name.as_str(), "helper");
    assert_eq!(agent.description.as_str(), "A test agent");
    assert_eq!(agent.tools.len(), 2);
    assert_eq!(agent.tools[0].as_str(), "remember");
    assert_eq!(agent.tools[1].as_str(), "search");
    assert_eq!(agent.skill_tags.len(), 1);
    assert_eq!(agent.skill_tags[0].as_str(), "coding");
    assert!(agent.system_prompt.contains("helpful coding assistant"));
    assert!(agent.system_prompt.contains("Be concise"));
}

#[test]
fn parse_cron_md_roundtrip() {
    let md = r#"---
name: daily-check
schedule: "0 0 9 * * *"
agent: assistant
---

Good morning! Please check for any pending tasks.
"#;
    let entry = parse_cron_md(md).unwrap();
    assert_eq!(entry.name.as_str(), "daily-check");
    assert_eq!(entry.schedule, "0 0 9 * * *");
    assert_eq!(entry.agent.as_str(), "assistant");
    assert!(entry.message.contains("Good morning"));
}

#[test]
fn load_agents_dir_discovers_md() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("assistant.md"),
        "---\nname: assistant\n---\nYou are helpful.\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("coder.md"),
        "---\nname: coder\ntools:\n  - remember\n---\nYou write code.\n",
    )
    .unwrap();
    let agents = load_agents_dir(dir.path()).unwrap();
    assert_eq!(agents.len(), 2);
    // Sorted by filename
    assert_eq!(agents[0].name.as_str(), "assistant");
    assert_eq!(agents[1].name.as_str(), "coder");
}

#[test]
fn load_agents_dir_ignores_non_md() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("assistant.md"),
        "---\nname: assistant\n---\nYou are helpful.\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("README.txt"), "not a skill").unwrap();
    let agents = load_agents_dir(dir.path()).unwrap();
    assert_eq!(agents.len(), 1);
}

#[test]
fn load_cron_dir_discovers_md() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("daily.md"),
        "---\nname: daily\nschedule: \"0 0 9 * * *\"\nagent: assistant\n---\nGood morning!\n",
    )
    .unwrap();
    let entries = load_cron_dir(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name.as_str(), "daily");
}

#[test]
fn load_agents_dir_missing_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nonexistent");
    let agents = load_agents_dir(&missing).unwrap();
    assert!(agents.is_empty());
}
