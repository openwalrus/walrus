//! BM25 session index, keyed by `(session_id, msg_idx)`. One doc per
//! indexed message. Per-role weighting is applied at search time to
//! the raw BM25 scores; the underlying inverted index stays kind-blind.
//!
//! `session_id` is an internal counter — agents and external callers
//! identify sessions by their `SessionHandle` (the storage slug). The
//! index maintains the `handle → session_id` mapping so live appends
//! and cold-start rebuilds resolve to the same internal id.

use super::hit::{
    MAX_HITS_PER_QUERY, MAX_SNIPPET_BYTES, MAX_WINDOW_ITEMS, SearchOptions, SessionHit, WindowItem,
};
use memory::bm25;
use std::collections::HashMap;
use wcore::model::{HistoryEntry, Role};
use wcore::storage::SessionHandle;

/// Document identity in the session index — one doc per message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageRef {
    pub session_id: u64,
    pub msg_idx: u32,
}

/// Per-message metadata cached alongside the BM25 index. Holds the
/// snippet + role we need to rebuild a `WindowItem` without going
/// back to disk on every search hit.
#[derive(Debug, Clone)]
struct DocMeta {
    role: Role,
    snippet: String,
    truncated: bool,
    tool_name: Option<String>,
}

/// Per-session metadata — handle and basic identity, used to populate
/// `SessionHit` without a storage lookup.
#[derive(Debug, Clone)]
struct SessionMeta {
    handle: SessionHandle,
    agent: String,
    sender: String,
    title: String,
    created_at: String,
    updated_at: String,
}

pub struct SessionIndex {
    bm25: bm25::Index<MessageRef>,
    docs: HashMap<MessageRef, DocMeta>,
    by_session: HashMap<u64, Vec<u32>>,
    sessions: HashMap<u64, SessionMeta>,
    handle_to_id: HashMap<String, u64>,
    next_id: u64,
}

impl Default for SessionIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionIndex {
    pub fn new() -> Self {
        Self {
            bm25: bm25::Index::new(),
            docs: HashMap::new(),
            by_session: HashMap::new(),
            sessions: HashMap::new(),
            handle_to_id: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn message_count(&self) -> usize {
        self.docs.len()
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Look up the internal `session_id` for a storage handle. Returns
    /// `None` if the session has never been registered.
    pub fn handle_to_session_id(&self, handle: &str) -> Option<&u64> {
        self.handle_to_id.get(handle)
    }

    /// Register or look up a session by its storage handle. Idempotent
    /// — repeat calls return the same `session_id`. Mutable fields
    /// (`title`, `updated_at`) are refreshed on every call so the
    /// index stays in step with the meta line.
    pub fn ensure_session(
        &mut self,
        handle: &SessionHandle,
        agent: &str,
        sender: &str,
        title: &str,
        created_at: &str,
        updated_at: &str,
    ) -> u64 {
        if let Some(&id) = self.handle_to_id.get(handle.as_str()) {
            if let Some(meta) = self.sessions.get_mut(&id) {
                if !title.is_empty() {
                    meta.title = title.to_owned();
                }
                if !updated_at.is_empty() {
                    meta.updated_at = updated_at.to_owned();
                }
            }
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.handle_to_id.insert(handle.as_str().to_owned(), id);
        self.sessions.insert(
            id,
            SessionMeta {
                handle: handle.clone(),
                agent: agent.to_owned(),
                sender: sender.to_owned(),
                title: title.to_owned(),
                created_at: created_at.to_owned(),
                updated_at: updated_at.to_owned(),
            },
        );
        id
    }

    /// Drop a session from the index. Used when a session is deleted
    /// or before re-indexing after compaction.
    pub fn forget_session(&mut self, session_id: u64) {
        if let Some(idxs) = self.by_session.remove(&session_id) {
            for msg_idx in idxs {
                let key = MessageRef {
                    session_id,
                    msg_idx,
                };
                self.bm25.remove(key);
                self.docs.remove(&key);
            }
        }
        if let Some(meta) = self.sessions.remove(&session_id) {
            self.handle_to_id.remove(meta.handle.as_str());
        }
    }

    /// Index one message. Auto-injected entries are skipped — they
    /// never reach storage so they shouldn't reach the index. Returns
    /// the assigned `msg_idx` (the message's position within this
    /// session's indexed log).
    pub fn insert_message(&mut self, session_id: u64, entry: &HistoryEntry) -> Option<u32> {
        if entry.auto_injected {
            return None;
        }
        let msg_idx = self.by_session.entry(session_id).or_default().len() as u32;
        let terms = bm25::tokenize(&extract_text(entry));
        let key = MessageRef {
            session_id,
            msg_idx,
        };
        self.bm25.insert(key, &terms);
        let (snippet, truncated) = make_snippet(entry);
        self.docs.insert(
            key,
            DocMeta {
                role: entry.role().clone(),
                snippet,
                truncated,
                tool_name: extract_tool_name(entry),
            },
        );
        self.by_session
            .get_mut(&session_id)
            .expect("by_session entry created above")
            .push(msg_idx);
        Some(msg_idx)
    }

    /// Run a search and shape hits with windowed context. Limits clamp
    /// to `MAX_HITS_PER_QUERY` and `MAX_WINDOW_ITEMS`.
    pub fn search(&self, query: &str, opts: &SearchOptions) -> Vec<SessionHit> {
        let limit = opts.limit.clamp(1, MAX_HITS_PER_QUERY);
        let context_before = opts.context_before;
        let context_after = opts.context_after;

        // Pull more raw hits than we'll keep — role-weighting can
        // reshuffle top-K, and filters drop arbitrary candidates.
        let raw_limit = (limit * 4).clamp(limit, MAX_HITS_PER_QUERY * 4);
        let raw = self.bm25.search(query, raw_limit);

        let mut hits: Vec<SessionHit> = Vec::with_capacity(raw.len());
        let mut seen: HashMap<u64, ()> = HashMap::new();
        for (mref, score) in raw {
            let Some(session) = self.sessions.get(&mref.session_id) else {
                continue;
            };
            if let Some(ref a) = opts.agent_filter
                && session.agent != *a
            {
                continue;
            }
            if let Some(ref s) = opts.sender_filter
                && session.sender != *s
            {
                continue;
            }
            // Best-hit-per-session: a second hit on the same session
            // is more "context noise" than signal at this limit —
            // windowing already covers nearby messages.
            if seen.insert(mref.session_id, ()).is_some() {
                continue;
            }
            let Some(meta) = self.docs.get(&mref) else {
                continue;
            };
            let weighted = score * role_weight(&meta.role, meta.tool_name.is_some());
            let window = self.build_window(mref, context_before, context_after);
            hits.push(SessionHit {
                session_id: mref.session_id,
                session_handle: Some(session.handle.clone()),
                msg_idx: mref.msg_idx,
                score: weighted,
                title: session.title.clone(),
                agent: session.agent.clone(),
                sender: session.sender.clone(),
                created_at: session.created_at.clone(),
                updated_at: session.updated_at.clone(),
                window,
            });
            if hits.len() >= limit {
                break;
            }
        }
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits
    }

    fn build_window(&self, anchor: MessageRef, before: usize, after: usize) -> Vec<WindowItem> {
        let Some(idxs) = self.by_session.get(&anchor.session_id) else {
            return Vec::new();
        };
        // `idxs` is the insertion-order list of message indices for
        // this session — append-only, so monotonically increasing.
        let pos = idxs
            .iter()
            .position(|i| *i == anchor.msg_idx)
            .unwrap_or(idxs.len().saturating_sub(1));
        let start = pos.saturating_sub(before);
        let end = (pos + after + 1).min(idxs.len());
        let mut span: Vec<u32> = idxs[start..end].to_vec();
        if span.len() > MAX_WINDOW_ITEMS {
            // Drop from the edges, keeping the anchor centred.
            let overflow = span.len() - MAX_WINDOW_ITEMS;
            let drop_before = overflow / 2;
            let drop_after = overflow - drop_before;
            span = span[drop_before..span.len() - drop_after].to_vec();
        }
        span.into_iter()
            .filter_map(|msg_idx| {
                let key = MessageRef {
                    session_id: anchor.session_id,
                    msg_idx,
                };
                let meta = self.docs.get(&key)?;
                Some(WindowItem {
                    role: meta.role.clone(),
                    msg_idx,
                    snippet: meta.snippet.clone(),
                    truncated: meta.truncated,
                    tool_name: meta.tool_name.clone(),
                })
            })
            .collect()
    }
}

fn extract_text(entry: &HistoryEntry) -> String {
    let text = entry.text();
    if !text.is_empty() {
        return text.to_owned();
    }
    // Tool-call assistant: index function name + arguments so callers
    // can find the conversation by what was invoked.
    if !entry.tool_calls().is_empty() {
        return entry
            .tool_calls()
            .iter()
            .map(|tc| format!("{} {}", tc.function.name, tc.function.arguments))
            .collect::<Vec<_>>()
            .join(" ");
    }
    String::new()
}

fn make_snippet(entry: &HistoryEntry) -> (String, bool) {
    let raw = extract_text(entry);
    if raw.len() <= MAX_SNIPPET_BYTES {
        return (raw, false);
    }
    let mut end = MAX_SNIPPET_BYTES;
    while end > 0 && !raw.is_char_boundary(end) {
        end -= 1;
    }
    (raw[..end].to_owned(), true)
}

fn extract_tool_name(entry: &HistoryEntry) -> Option<String> {
    if matches!(entry.role(), Role::Tool) {
        return entry.message.name.clone().filter(|s| !s.is_empty());
    }
    let calls = entry.tool_calls();
    if calls.is_empty() {
        return None;
    }
    Some(calls[0].function.name.clone())
}

/// Per-role / per-kind score multiplier. Defaults from the community
/// Claude Code conversation-search pattern (alexop.dev,
/// `raine/claude-history`); see RFC 0185 for the calibration plan.
fn role_weight(role: &Role, has_tool_call: bool) -> f64 {
    match role {
        Role::User => 1.5,
        Role::Assistant if has_tool_call => 1.3,
        Role::Assistant => 1.0,
        Role::Tool => 1.3,
        Role::System => 0.5,
        _ => 1.0,
    }
}
