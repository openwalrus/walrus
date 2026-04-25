//! Session search index — direct unit/integration tests on the
//! `SessionIndex` API. Doesn't go through `Runtime` because the index
//! is a pure data structure and we want fast, deterministic coverage
//! of the lexical-recall behavior.

use crabtalk_runtime::sessions::{SearchOptions, SessionIndex};
use wcore::{
    model::{HistoryEntry, Role},
    storage::SessionHandle,
};

fn h(slug: &str) -> SessionHandle {
    SessionHandle::new(slug)
}

fn ensure(index: &mut SessionIndex, handle: &SessionHandle) -> u64 {
    index.ensure_session(
        handle,
        "crab",
        "tester",
        handle.as_str(),
        "2026-04-25T00:00:00Z",
        "2026-04-25T00:00:00Z",
    )
}

#[test]
fn search_returns_hit_with_handle_and_window() {
    let mut idx = SessionIndex::new();
    let handle = h("crab_tester_1");
    let sid = ensure(&mut idx, &handle);
    idx.insert_message(sid, &HistoryEntry::user("planning the cron refactor"));
    idx.insert_message(
        sid,
        &HistoryEntry::assistant("OK, splitting the daemon.", None, None),
    );

    let hits = idx.search("cron", &SearchOptions::default());
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    assert_eq!(hit.session_handle.as_ref().unwrap(), &handle);
    assert!(!hit.window.is_empty());
    assert!(hit.window.iter().any(|w| w.snippet.contains("cron")));
}

#[test]
fn search_skips_auto_injected_entries() {
    let mut idx = SessionIndex::new();
    let handle = h("crab_tester_2");
    let sid = ensure(&mut idx, &handle);
    let mut env_block = HistoryEntry::user("crontab schedules");
    env_block.auto_injected = true;
    idx.insert_message(sid, &env_block);
    idx.insert_message(sid, &HistoryEntry::user("unrelated chatter"));

    let hits = idx.search("crontab", &SearchOptions::default());
    assert!(
        hits.is_empty(),
        "auto-injected env block must not be indexed"
    );
}

#[test]
fn agent_filter_excludes_other_agents() {
    let mut idx = SessionIndex::new();
    let other = h("crab_other_1");
    let sid_other = idx.ensure_session(
        &other,
        "crab",
        "other",
        "",
        "2026-04-25T00:00:00Z",
        "2026-04-25T00:00:00Z",
    );
    idx.insert_message(sid_other, &HistoryEntry::user("matching keyword foobar"));

    let mine = h("crab_tester_3");
    let sid_mine = idx.ensure_session(
        &mine,
        "crab",
        "tester",
        "",
        "2026-04-25T00:00:00Z",
        "2026-04-25T00:00:00Z",
    );
    idx.insert_message(sid_mine, &HistoryEntry::user("matching keyword foobar"));

    let opts = SearchOptions {
        sender_filter: Some("tester".to_owned()),
        ..SearchOptions::default()
    };

    let hits = idx.search("foobar", &opts);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].session_handle.as_ref().unwrap(), &mine);
}

#[test]
fn user_messages_outweigh_assistant_for_same_match() {
    let mut idx = SessionIndex::new();
    let user_session = h("u_user_1");
    let asst_session = h("u_asst_1");
    let su = ensure(&mut idx, &user_session);
    let sa = ensure(&mut idx, &asst_session);
    idx.insert_message(su, &HistoryEntry::user("the deploy pipeline failed"));
    idx.insert_message(
        sa,
        &HistoryEntry::assistant("the deploy pipeline failed", None, None),
    );

    let hits = idx.search("deploy pipeline", &SearchOptions::default());
    assert!(hits.len() >= 2);
    assert_eq!(
        hits[0].session_handle.as_ref().unwrap(),
        &user_session,
        "user role weight should rank above assistant for identical content"
    );
}

#[test]
fn forget_session_removes_postings() {
    let mut idx = SessionIndex::new();
    let handle = h("crab_tester_4");
    let sid = ensure(&mut idx, &handle);
    idx.insert_message(sid, &HistoryEntry::user("zebra crossing"));
    assert_eq!(idx.search("zebra", &SearchOptions::default()).len(), 1);

    idx.forget_session(sid);
    assert!(idx.search("zebra", &SearchOptions::default()).is_empty());
    assert_eq!(idx.session_count(), 0);
    assert_eq!(idx.message_count(), 0);
}

#[test]
fn long_message_snippet_is_truncated() {
    let mut idx = SessionIndex::new();
    let handle = h("crab_tester_5");
    let sid = ensure(&mut idx, &handle);
    let big = "elephant ".repeat(500); // >> MAX_SNIPPET_BYTES
    idx.insert_message(sid, &HistoryEntry::user(big));

    let hits = idx.search("elephant", &SearchOptions::default());
    assert_eq!(hits.len(), 1);
    let item = &hits[0].window[0];
    assert!(item.truncated);
    assert!(item.snippet.len() <= 1024);
}

#[test]
fn ensure_session_is_idempotent_and_refreshes_meta() {
    let mut idx = SessionIndex::new();
    let handle = h("crab_tester_6");
    let id1 = idx.ensure_session(
        &handle,
        "crab",
        "tester",
        "old-title",
        "2026-04-25T00:00:00Z",
        "2026-04-25T00:00:00Z",
    );
    let id2 = idx.ensure_session(
        &handle,
        "crab",
        "tester",
        "new-title",
        "2026-04-25T00:00:00Z",
        "2026-04-25T00:00:01Z",
    );
    assert_eq!(id1, id2);
    idx.insert_message(id1, &HistoryEntry::user("anchor message"));

    let hits = idx.search("anchor", &SearchOptions::default());
    assert_eq!(hits[0].title, "new-title");
    assert_eq!(hits[0].updated_at, "2026-04-25T00:00:01Z");
}

#[test]
fn role_filtering_via_helpers() {
    use wcore::model::ToolCall;
    // Tool-call assistant indexes the function name + args.
    let mut idx = SessionIndex::new();
    let handle = h("crab_tester_7");
    let sid = ensure(&mut idx, &handle);
    let call = ToolCall {
        index: None,
        id: "c1".into(),
        kind: wcore::model::ToolType::Function,
        function: wcore::model::FunctionCall {
            name: "shell".into(),
            arguments: r#"{"command": "rebuild_index"}"#.into(),
        },
    };
    idx.insert_message(sid, &HistoryEntry::assistant("", None, Some(&[call])));

    let hits = idx.search("rebuild_index", &SearchOptions::default());
    assert_eq!(hits.len(), 1, "tool-call args should be searchable");
    let item = &hits[0].window[0];
    assert!(matches!(item.role, Role::Assistant));
    assert_eq!(item.tool_name.as_deref(), Some("shell"));
}
