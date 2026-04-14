use crabtalk_memory::{EntryKind, Memory, Op};
use std::fs;
use tempfile::tempdir;

fn add(mem: &mut Memory, kind: EntryKind, name: &str, content: &str, aliases: &[&str]) {
    mem.apply(Op::Add {
        name: name.to_owned(),
        content: content.to_owned(),
        aliases: aliases.iter().map(|s| (*s).to_owned()).collect(),
        kind,
    })
    .unwrap();
}

#[test]
fn dump_creates_sections_and_summary() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    let mut mem = Memory::new();
    add(
        &mut mem,
        EntryKind::Note,
        "rust-style",
        "group imports",
        &[],
    );
    add(
        &mut mem,
        EntryKind::Archive,
        "20260401-session42",
        "compacted summary",
        &[],
    );
    add(
        &mut mem,
        EntryKind::Prompt,
        "global",
        "- [rust-style](rust-style.md)",
        &[],
    );

    mem.dump(&brain).unwrap();

    assert!(brain.join("notes/rust-style.md").is_file());
    assert!(brain.join("archives/20260401-session42.md").is_file());
    assert!(brain.join("prompts/global.md").is_file());

    let summary = fs::read_to_string(brain.join("SUMMARY.md")).unwrap();
    assert!(summary.contains("# Notes"));
    assert!(summary.contains("# Archives"));
    assert!(summary.contains("# Prompts"));
    assert!(summary.contains("[rust-style](notes/rust-style.md)"));
    assert!(summary.contains("[20260401-session42](archives/20260401-session42.md)"));
    assert!(summary.contains("[global](prompts/global.md)"));
}

#[test]
fn dump_emits_refs_section_only_when_aliases_exist() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    let mut mem = Memory::new();
    add(&mut mem, EntryKind::Note, "plain", "just prose", &[]);
    add(
        &mut mem,
        EntryKind::Note,
        "tagged",
        "prod rollout steps",
        &["ship", "release"],
    );

    mem.dump(&brain).unwrap();

    let plain = fs::read_to_string(brain.join("notes/plain.md")).unwrap();
    assert!(!plain.contains("## Refs"));
    assert_eq!(plain.trim(), "just prose");

    let tagged = fs::read_to_string(brain.join("notes/tagged.md")).unwrap();
    assert!(tagged.contains("## Refs"));
    assert!(tagged.contains("- ship"));
    assert!(tagged.contains("- release"));
}

#[test]
fn round_trip_preserves_content_and_aliases() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    let mut mem = Memory::new();
    add(&mut mem, EntryKind::Note, "a", "line one\nline two", &[]);
    add(
        &mut mem,
        EntryKind::Archive,
        "arc",
        "session text",
        &["session-42", "compact"],
    );
    add(
        &mut mem,
        EntryKind::Prompt,
        "global",
        "curated overview",
        &[],
    );
    mem.dump(&brain).unwrap();

    let mut loaded = Memory::new();
    loaded.load(&brain).unwrap();

    assert_eq!(loaded.list().count(), 3);

    let a = loaded.get("a").unwrap();
    assert_eq!(a.content, "line one\nline two");
    assert_eq!(a.kind, EntryKind::Note);
    assert!(a.aliases.is_empty());

    let arc = loaded.get("arc").unwrap();
    assert_eq!(arc.kind, EntryKind::Archive);
    assert_eq!(arc.aliases, vec!["session-42", "compact"]);

    let global = loaded.get("global").unwrap();
    assert_eq!(global.kind, EntryKind::Prompt);
}

#[test]
fn load_rebuilds_search_index() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    let mut mem = Memory::new();
    add(
        &mut mem,
        EntryKind::Note,
        "fox",
        "quick brown fox",
        &["animal"],
    );
    mem.dump(&brain).unwrap();

    let mut loaded = Memory::new();
    loaded.load(&brain).unwrap();
    let hits = loaded.search("animal", 5);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entry.name, "fox");
}

#[test]
fn load_replaces_existing_entries() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    let mut src = Memory::new();
    add(&mut src, EntryKind::Note, "keeper", "new content", &[]);
    src.dump(&brain).unwrap();

    let mut mem = Memory::new();
    add(&mut mem, EntryKind::Note, "goner", "old content", &[]);
    mem.load(&brain).unwrap();

    assert!(mem.get("goner").is_none());
    assert!(mem.get("keeper").is_some());
}

#[test]
fn load_ignores_summary_and_non_markdown() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    fs::create_dir_all(brain.join("notes")).unwrap();
    fs::write(brain.join("SUMMARY.md"), "# Summary").unwrap();
    fs::write(brain.join("notes/real.md"), "real note").unwrap();
    fs::write(brain.join("notes/ignored.txt"), "not markdown").unwrap();

    let mut mem = Memory::new();
    mem.load(&brain).unwrap();
    assert_eq!(mem.list().count(), 1);
    assert!(mem.get("real").is_some());
}

#[test]
fn refs_heading_mid_content_is_respected() {
    // A file that has `## Refs` both as content text and as a trailing
    // section. The trailing one wins; the earlier occurrence stays in
    // the content.
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    fs::create_dir_all(brain.join("notes")).unwrap();
    fs::write(
        brain.join("notes/quirky.md"),
        "prose mentioning ## Refs inline\nmore prose\n\n## Refs\n\n- real-alias\n",
    )
    .unwrap();

    let mut mem = Memory::new();
    mem.load(&brain).unwrap();
    let e = mem.get("quirky").unwrap();
    assert!(e.content.contains("## Refs inline"));
    assert_eq!(e.aliases, vec!["real-alias"]);
}

#[test]
fn entry_without_refs_section_has_no_aliases() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    fs::create_dir_all(brain.join("notes")).unwrap();
    fs::write(brain.join("notes/plain.md"), "hello world\n").unwrap();

    let mut mem = Memory::new();
    mem.load(&brain).unwrap();
    let e = mem.get("plain").unwrap();
    assert_eq!(e.content, "hello world");
    assert!(e.aliases.is_empty());
}

#[test]
fn alias_verbatim_parses_annotations() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    fs::create_dir_all(brain.join("notes")).unwrap();
    fs::write(
        brain.join("notes/annotated.md"),
        "content\n\n## Refs\n\n- ship (legacy)\n- release\n",
    )
    .unwrap();

    let mut mem = Memory::new();
    mem.load(&brain).unwrap();
    assert_eq!(
        mem.get("annotated").unwrap().aliases,
        vec!["ship (legacy)", "release"]
    );
}

#[test]
fn redump_clears_orphans() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");

    let mut mem = Memory::new();
    add(&mut mem, EntryKind::Note, "alpha", "first", &[]);
    add(&mut mem, EntryKind::Note, "beta", "second", &[]);
    mem.dump(&brain).unwrap();
    assert!(brain.join("notes/alpha.md").exists());
    assert!(brain.join("notes/beta.md").exists());

    mem.apply(Op::Remove {
        name: "alpha".into(),
    })
    .unwrap();
    mem.dump(&brain).unwrap();

    assert!(!brain.join("notes/alpha.md").exists());
    assert!(brain.join("notes/beta.md").exists());
}

#[test]
fn load_duplicate_leaves_state_untouched() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    fs::create_dir_all(brain.join("notes")).unwrap();
    fs::create_dir_all(brain.join("archives")).unwrap();
    fs::write(brain.join("notes/dup.md"), "a").unwrap();
    fs::write(brain.join("archives/dup.md"), "b").unwrap();

    let mut mem = Memory::new();
    add(&mut mem, EntryKind::Note, "survivor", "stays put", &[]);
    assert!(mem.load(&brain).is_err());
    // Failed load must not have wiped the existing entry.
    assert!(mem.get("survivor").is_some());
}

#[test]
fn invalid_name_rejected_by_dump() {
    let dir = tempdir().unwrap();
    let brain = dir.path().join("brain");
    let mut mem = Memory::new();
    add(&mut mem, EntryKind::Note, "bad/name", "x", &[]);
    assert!(mem.dump(&brain).is_err());
}

#[test]
fn persistent_db_dumps_and_reloads_through_tree() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("mem.db");
    let brain = dir.path().join("brain");

    let mut mem = Memory::open(&db).unwrap();
    add(&mut mem, EntryKind::Note, "persistent", "stays", &["alias"]);
    mem.dump(&brain).unwrap();
    mem.load(&brain).unwrap();

    let reopened = Memory::open(&db).unwrap();
    let e = reopened.get("persistent").unwrap();
    assert_eq!(e.content, "stays");
    assert_eq!(e.aliases, vec!["alias"]);
}
