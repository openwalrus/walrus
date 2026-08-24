//! Sessions, one directory each: metadata, messages, and the trace.
//!
//! A message's index is its line number, which is what lets a hit point
//! at one and a window read around it without an index to maintain.

use crate::backend::{
    self, Backend,
    search::{self, Doc},
};
use anyhow::Result;
use std::{collections::BTreeMap, path::PathBuf};
use store::{
    AgentId, EventLine, HistoryEntry, MAX_HITS_PER_QUERY, MAX_WINDOW_ITEMS, SearchOptions,
    SessionHandle, SessionHit, SessionMeta, SessionSnapshot, Weights, WindowItem,
    interface::{Agents, Sessions},
};

impl Backend {
    fn session_dir(&self, handle: &SessionHandle) -> PathBuf {
        self.sessions_dir().join(backend::encode(handle.as_str()))
    }

    fn meta_path(&self, handle: &SessionHandle) -> PathBuf {
        self.session_dir(handle).join("meta.json")
    }

    fn messages_path(&self, handle: &SessionHandle) -> PathBuf {
        self.session_dir(handle).join("messages.jsonl")
    }

    fn events_path(&self, handle: &SessionHandle) -> PathBuf {
        self.session_dir(handle).join("events.jsonl")
    }

    fn archive_path(&self, handle: &SessionHandle) -> PathBuf {
        self.session_dir(handle).join("archive")
    }

    async fn meta(&self, handle: &SessionHandle) -> Result<Option<SessionMeta>> {
        backend::read_json(&self.meta_path(handle)).await
    }

    async fn messages(&self, handle: &SessionHandle) -> Result<Vec<HistoryEntry>> {
        backend::read_lines(&self.messages_path(handle)).await
    }

    async fn handles(&self) -> Result<Vec<SessionHandle>> {
        let Ok(mut entries) = tokio::fs::read_dir(self.sessions_dir()).await else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().into_owned();
            out.push(SessionHandle::new(backend::decode(&name)));
        }
        out.sort();
        Ok(out)
    }

    /// The messages surrounding a hit, bounded by [`MAX_WINDOW_ITEMS`].
    fn window(&self, history: &[HistoryEntry], at: usize, opts: &SearchOptions) -> Vec<WindowItem> {
        let before = opts.context_before.min(MAX_WINDOW_ITEMS);
        let after = opts.context_after.min(MAX_WINDOW_ITEMS);
        let first = at.saturating_sub(before);
        let last = at
            .saturating_add(after)
            .min(history.len().saturating_sub(1));
        let mut out = Vec::new();
        for (idx, entry) in history.iter().enumerate().take(last + 1).skip(first) {
            if out.len() >= MAX_WINDOW_ITEMS {
                break;
            }
            let (snippet, truncated) = entry.snippet();
            out.push(WindowItem {
                role: entry.role().clone(),
                msg_idx: idx as u32,
                snippet,
                truncated,
                tool_name: entry.tool_name(),
            });
        }
        out
    }
}

impl Sessions for Backend {
    async fn create_session(
        &self,
        handle: &SessionHandle,
        agent: &AgentId,
        created_by: &str,
        root: Option<PathBuf>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let meta = SessionMeta {
            agent: *agent,
            created_by: created_by.to_owned(),
            created_at: now.clone(),
            title: String::new(),
            updated_at: now,
            message_count: 0,
            summary: None,
            root,
        };
        backend::write_json(&self.meta_path(handle), &meta).await
    }

    async fn load_session(&self, handle: &SessionHandle) -> Result<Option<SessionSnapshot>> {
        let Some(meta) = self.meta(handle).await? else {
            return Ok(None);
        };
        Ok(Some(SessionSnapshot {
            meta,
            history: self.messages(handle).await?,
            archive: tokio::fs::read_to_string(self.archive_path(handle))
                .await
                .ok()
                .map(|name| name.trim().to_owned()),
        }))
    }

    async fn list_sessions(&self) -> Result<Vec<(SessionHandle, SessionMeta)>> {
        let mut out = Vec::new();
        for handle in self.handles().await? {
            if let Some(meta) = self.meta(&handle).await? {
                out.push((handle, meta));
            }
        }
        out.sort_by(|(ah, am), (bh, bm)| {
            bm.updated_at
                .cmp(&am.updated_at)
                .then_with(|| bm.created_at.cmp(&am.created_at))
                .then_with(|| ah.as_str().cmp(bh.as_str()))
        });
        Ok(out)
    }

    async fn append_session_messages(
        &self,
        handle: &SessionHandle,
        entries: &[HistoryEntry],
    ) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let Some(mut meta) = self.meta(handle).await? else {
            anyhow::bail!("session not found: {}", handle.as_str());
        };
        backend::append_json(&self.messages_path(handle), entries).await?;
        meta.message_count += entries.len() as u64;
        meta.updated_at = chrono::Utc::now().to_rfc3339();
        backend::write_json(&self.meta_path(handle), &meta).await
    }

    async fn append_session_events(
        &self,
        handle: &SessionHandle,
        events: &[EventLine],
    ) -> Result<()> {
        backend::append_json(&self.events_path(handle), events).await
    }

    /// The compacted prefix leaves the live history: the marker points at
    /// where the text went, so the messages it covers are dropped rather
    /// than kept beside it.
    async fn append_session_compact(&self, handle: &SessionHandle, archive: &str) -> Result<()> {
        self.truncate_session_messages(handle, 0).await?;
        Ok(tokio::fs::write(self.archive_path(handle), archive).await?)
    }

    async fn truncate_session_messages(&self, handle: &SessionHandle, keep: usize) -> Result<()> {
        let Some(mut meta) = self.meta(handle).await? else {
            return Ok(());
        };
        let mut history = self.messages(handle).await?;
        let kept = keep.min(history.len());
        history.truncate(kept);
        backend::write_lines(&self.messages_path(handle), &history).await?;
        meta.message_count = kept as u64;
        backend::write_json(&self.meta_path(handle), &meta).await
    }

    /// `message_count` is maintained by append and truncate, so the
    /// stored value wins over whatever arrives here.
    async fn update_session_meta(&self, handle: &SessionHandle, meta: &SessionMeta) -> Result<()> {
        let mut meta = meta.clone();
        if let Some(stored) = self.meta(handle).await? {
            meta.message_count = stored.message_count;
        }
        backend::write_json(&self.meta_path(handle), &meta).await
    }

    async fn delete_session(&self, handle: &SessionHandle) -> Result<bool> {
        Ok(tokio::fs::remove_dir_all(self.session_dir(handle))
            .await
            .is_ok())
    }

    async fn delete_sessions_of(&self, agent: &AgentId) -> Result<usize> {
        let mut purged = 0;
        for (handle, meta) in self.list_sessions().await? {
            if meta.agent == *agent && self.delete_session(&handle).await? {
                purged += 1;
            }
        }
        Ok(purged)
    }

    /// Rank messages, keep each session's best, then boost by what the
    /// session as a whole says about itself.
    async fn search_sessions(&self, query: &str, opts: &SearchOptions) -> Result<Vec<SessionHit>> {
        let weights = Weights::default();
        let limit = opts.limit.clamp(1, MAX_HITS_PER_QUERY);
        let candidates = limit.saturating_mul(weights.candidates_per_hit);

        // One pass over the corpus builds both the message documents and
        // the metadata ones, so a search reads each session once.
        let mut loaded: Vec<(SessionHandle, SessionMeta, Vec<HistoryEntry>)> = Vec::new();
        let (mut docs, mut located) = (Vec::new(), Vec::new());
        let (mut meta_docs, mut meta_located) = (Vec::new(), Vec::new());
        for (handle, meta) in self.list_sessions().await? {
            let history = self.messages(&handle).await?;
            let at = loaded.len();
            for (idx, entry) in history.iter().enumerate() {
                let Some((body, role)) = entry.indexable() else {
                    continue;
                };
                let weight = match role {
                    "user" => weights.user,
                    "assistant_tool" => weights.tool_call,
                    _ => 1.0,
                };
                docs.push(Doc { text: body, weight });
                located.push((at, idx));
            }
            if !meta.title.is_empty() {
                meta_docs.push(Doc {
                    text: meta.title.clone(),
                    weight: 1.0,
                });
                meta_located.push((at, weights.title_boost));
            }
            if let Some(summary) = &meta.summary {
                meta_docs.push(Doc {
                    text: summary.clone(),
                    weight: 1.0,
                });
                meta_located.push((at, weights.summary_boost));
            }
            loaded.push((handle, meta, history));
        }

        // Best message per session.
        let mut best: BTreeMap<usize, (usize, f64)> = BTreeMap::new();
        for (doc, score) in search::rank(&docs, query, candidates) {
            let (at, idx) = located[doc];
            best.entry(at)
                .and_modify(|held| {
                    if score > held.1 {
                        *held = (idx, score);
                    }
                })
                .or_insert((idx, score));
        }
        if best.is_empty() {
            return Ok(Vec::new());
        }

        let mut boosts: BTreeMap<usize, f64> = BTreeMap::new();
        for (doc, _) in search::rank(&meta_docs, query, candidates) {
            let (at, boost) = meta_located[doc];
            *boosts.entry(at).or_insert(0.0) += boost;
        }

        let mut hits = Vec::new();
        for (at, (idx, score)) in best {
            let (handle, meta, history) = &loaded[at];
            if opts.agent_filter.is_some_and(|id| meta.agent != id) {
                continue;
            }
            if opts
                .sender_filter
                .as_ref()
                .is_some_and(|s| &meta.created_by != s)
            {
                continue;
            }
            // A hit is read by a person or a model, and a bare ULID names
            // nothing.
            let agent_name = self
                .load_agent(&meta.agent)
                .await?
                .map(|config| config.name)
                .unwrap_or_default();
            hits.push(SessionHit {
                session_handle: handle.clone(),
                msg_idx: idx as u32,
                score: score + boosts.get(&at).copied().unwrap_or(0.0),
                title: meta.title.clone(),
                agent: meta.agent,
                agent_name,
                sender: meta.created_by.clone(),
                created_at: meta.created_at.clone(),
                updated_at: meta.updated_at.clone(),
                window: self.window(history, idx, opts),
            });
        }
        hits.sort_by(|a, b| b.score.total_cmp(&a.score));
        hits.truncate(limit);
        Ok(hits)
    }
}
