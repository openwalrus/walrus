//! MemoryService — standalone graph-based memory service owning LanceDB
//! entity, relation, and journal storage with candle embeddings.

use crate::{
    config::MemoryConfig,
    embedder::Embedder,
    lance::{Direction, EntityRow, LanceStore, RelationRow},
};
use std::{path::Path, sync::Mutex};
const MEMORY_PROMPT: &str = include_str!("../prompts/memory.md");

/// Graph-based memory service owning LanceDB entity, relation, and journal storage.
pub struct MemoryService {
    pub lance: LanceStore,
    pub embedder: Mutex<Embedder>,
    pub auto_recall: bool,
}

impl MemoryService {
    /// Create a new MemoryService, opening or creating the LanceDB database.
    pub async fn open(memory_dir: impl AsRef<Path>, config: &MemoryConfig) -> anyhow::Result<Self> {
        let memory_dir = memory_dir.as_ref();
        tokio::fs::create_dir_all(memory_dir).await?;

        // Load embedder first — needed for entity vector backfill during open.
        let cache_dir = wcore::paths::CONFIG_DIR.join(".cache").join("huggingface");
        let embedder = tokio::task::spawn_blocking(move || Embedder::load(&cache_dir)).await??;

        let lance_dir = memory_dir.join("lance");
        let embed_mutex = Mutex::new(embedder);
        let lance = LanceStore::open(&lance_dir, |text| {
            let mut emb = embed_mutex
                .lock()
                .map_err(|e| anyhow::anyhow!("embedder lock poisoned: {e}"))?;
            emb.embed(text)
        })
        .await?;

        Ok(Self {
            lance,
            embedder: embed_mutex,
            auto_recall: config.auto_recall,
        })
    }

    /// Generate an embedding vector for text. Runs candle inference in a blocking task.
    pub async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let text = text.to_owned();
        tokio::task::block_in_place(|| {
            let mut embedder = self
                .embedder
                .lock()
                .map_err(|e| anyhow::anyhow!("embedder lock poisoned: {e}"))?;
            embedder.embed(&text)
        })
    }

    /// Return the memory prompt to append to agent system prompts.
    pub fn memory_prompt() -> &'static str {
        MEMORY_PROMPT
    }

    // ── Tool dispatch methods ────────────────────────────────────────

    /// Unified search: embed query → semantic entity search → 1-hop graph on top-3 → format.
    ///
    /// Shared by `dispatch_recall` (per query) and `handle_before_run` (auto-recall).
    /// Returns `None` when no results are found.
    pub async fn unified_search(&self, query: &str, limit: usize) -> Option<String> {
        let vector = match self.embed(query).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("embed failed for search: {e}");
                return None;
            }
        };

        let mut lines = Vec::new();

        // Semantic entity search.
        let entities = self
            .lance
            .search_entities_semantic(&vector, None, limit)
            .await
            .unwrap_or_default();
        for e in &entities {
            lines.push(format!("[{}] {}: {}", e.entity_type, e.key, e.value));
        }

        // 1-hop connections for top-3 matched entities.
        for e in entities.iter().take(3) {
            if let Ok(rels) = self
                .lance
                .find_connections(&e.id, None, Direction::Both, 5)
                .await
            {
                for r in &rels {
                    let line = format!("{} -[{}]-> {}", r.source, r.relation, r.target);
                    if !lines.contains(&line) {
                        lines.push(line);
                    }
                }
            }
        }

        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    /// Dispatch the `recall` tool call — batch queries via `unified_search`.
    pub async fn dispatch_recall(&self, args: &str) -> String {
        let input: crate::tool::Recall = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        if input.queries.is_empty() {
            return "missing required field: queries".to_owned();
        }
        let limit = input.limit.unwrap_or(5) as usize;

        let mut sections = Vec::new();
        for query in &input.queries {
            if query.is_empty() {
                continue;
            }
            if let Some(result) = self.unified_search(query, limit).await {
                sections.push(format!("## {query}\n{result}"));
            }
        }

        if sections.is_empty() {
            "no results found".to_owned()
        } else {
            sections.join("\n\n")
        }
    }

    /// Dispatch the `extract` tool call — batch upsert entities + relations.
    pub async fn dispatch_extract(&self, args: &str) -> String {
        let input: crate::tool::Extract = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };

        let mut results = Vec::new();

        // Upsert entities.
        for entity in &input.entities {
            if entity.key.is_empty() {
                results.push("skipped entity: empty key".to_owned());
                continue;
            }
            let entity_type = entity.entity_type.as_deref().unwrap_or("fact");
            let id = entity_id(entity_type, &entity.key);
            let text = format!("{} {}", entity.key, entity.value);
            let vector = match self.embed(&text).await {
                Ok(v) => v,
                Err(e) => {
                    results.push(format!("failed to embed '{}': {e}", entity.key));
                    continue;
                }
            };
            let row = EntityRow {
                id: &id,
                entity_type,
                key: &entity.key,
                value: &entity.value,
                vector,
            };
            match self.lance.upsert_entity(&row).await {
                Ok(()) => results.push(format!("stored [{}] {}", entity_type, entity.key)),
                Err(e) => results.push(format!("failed '{}': {e}", entity.key)),
            }
        }

        // Upsert relations.
        for rel in &input.relations {
            if rel.source.is_empty() || rel.target.is_empty() || rel.relation.is_empty() {
                results.push("skipped relation: empty field".to_owned());
                continue;
            }

            // Look up source entity.
            let source = match self.lance.find_entity_by_key(&rel.source).await {
                Ok(Some(e)) => e,
                Ok(None) => {
                    results.push(format!("source not found: '{}'", rel.source));
                    continue;
                }
                Err(e) => {
                    results.push(format!("source lookup failed: {e}"));
                    continue;
                }
            };

            // Look up target entity.
            let target = match self.lance.find_entity_by_key(&rel.target).await {
                Ok(Some(e)) => e,
                Ok(None) => {
                    results.push(format!("target not found: '{}'", rel.target));
                    continue;
                }
                Err(e) => {
                    results.push(format!("target lookup failed: {e}"));
                    continue;
                }
            };

            let row = RelationRow {
                source: &source.id,
                relation: &rel.relation,
                target: &target.id,
            };
            match self.lance.upsert_relation(&row).await {
                Ok(()) => results.push(format!(
                    "related: {} -[{}]-> {}",
                    rel.source, rel.relation, rel.target
                )),
                Err(e) => results.push(format!("relation failed: {e}")),
            }
        }

        if results.is_empty() {
            "nothing to extract".to_owned()
        } else {
            results.join("\n")
        }
    }

    /// Internal dispatch for storing a journal entry.
    ///
    /// Called by the agent loop after compaction — `args` is the raw summary text.
    pub async fn dispatch_journal(&self, args: &str, agent: &str) -> String {
        if args.is_empty() {
            return "empty journal entry".to_owned();
        }

        let vector = match self.embed(args).await {
            Ok(v) => v,
            Err(e) => return format!("failed to embed journal: {e}"),
        };

        match self.lance.insert_journal(agent, args, vector).await {
            Ok(()) => "journal entry stored".to_owned(),
            Err(e) => format!("failed to store journal: {e}"),
        }
    }
}

/// Build entity ID: `{entity_type}:{key}`.
fn entity_id(entity_type: &str, key: &str) -> String {
    format!("{entity_type}:{key}")
}
