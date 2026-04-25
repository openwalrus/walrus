//! Public session-search surface on `Runtime` — wraps the internal
//! [`SessionIndex`] with caller-friendly methods.

use super::Runtime;
use crate::{
    Config,
    sessions::{SearchOptions, SessionHit},
};
use wcore::storage::Storage;

impl<C: Config> Runtime<C> {
    /// Number of messages currently indexed.
    pub fn indexed_message_count(&self) -> usize {
        self.session_index.read().message_count()
    }

    /// Number of sessions currently registered in the index.
    pub fn indexed_session_count(&self) -> usize {
        self.session_index.read().session_count()
    }

    /// BM25 search over indexed conversation messages. Returns
    /// best-hit-per-session up to `opts.limit`, each with a windowed
    /// excerpt around the match. Limits clamp to the index's hard caps.
    pub fn search_sessions(&self, query: &str, opts: &SearchOptions) -> Vec<SessionHit> {
        self.session_index.read().search(query, opts)
    }

    /// Rebuild the session search index from storage. Called on
    /// startup; idempotent — safe to re-run, though it does discard
    /// in-flight live edits to the index.
    pub fn rebuild_session_index(&self) -> anyhow::Result<()> {
        let storage = self.storage();
        let summaries = storage.list_sessions()?;
        let mut index = self.session_index.write();
        *index = crate::sessions::SessionIndex::new();
        for summary in summaries {
            let Some(snapshot) = storage.load_session(&summary.handle)? else {
                continue;
            };
            let session_id = index.ensure_session(
                &summary.handle,
                &snapshot.meta.agent,
                &snapshot.meta.created_by,
                &snapshot.meta.title,
                snapshot.meta.summary.as_deref(),
                &snapshot.meta.created_at,
                &snapshot.meta.updated_at,
            );
            for entry in &snapshot.history {
                index.insert_message(session_id, entry);
            }
        }
        Ok(())
    }
}
