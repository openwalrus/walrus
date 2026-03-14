//! MemoryService — standalone graph-based memory service owning LanceDB
//! entity, relation, and journal storage with candle embeddings.

use crate::{
    config::MemoryConfig,
    embedder::Embedder,
    lance::{Direction, EntityRow, LanceStore, RelationRow},
};
use std::{path::Path, sync::Mutex};
const MEMORY_PROMPT: &str = include_str!("../prompts/memory.md");

/// Default entity types provided by the framework.
const DEFAULT_ENTITIES: &[&str] = &[
    "fact",
    "preference",
    "person",
    "event",
    "concept",
    "identity",
    "profile",
];

/// Default relation types provided by the framework.
const DEFAULT_RELATIONS: &[&str] = &[
    "knows",
    "prefers",
    "related_to",
    "caused_by",
    "part_of",
    "depends_on",
    "tagged_with",
];

/// Graph-based memory service owning LanceDB entity, relation, and journal storage.
pub struct MemoryService {
    pub lance: LanceStore,
    pub embedder: Mutex<Embedder>,
    pub allowed_entities: Vec<String>,
    pub allowed_relations: Vec<String>,
    pub connection_limit: usize,
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

        let allowed_entities = merge_defaults(DEFAULT_ENTITIES, &config.entities);
        let allowed_relations = merge_defaults(DEFAULT_RELATIONS, &config.relations);
        let connection_limit = config.connections.clamp(1, 100);

        Ok(Self {
            lance,
            embedder: embed_mutex,
            allowed_entities,
            allowed_relations,
            connection_limit,
            auto_recall: config.auto_recall,
        })
    }

    /// Check if an entity type is allowed.
    pub fn is_valid_entity(&self, entity_type: &str) -> bool {
        self.allowed_entities.iter().any(|t| t == entity_type)
    }

    /// Check if a relation type is allowed.
    pub fn is_valid_relation(&self, relation: &str) -> bool {
        self.allowed_relations.iter().any(|r| r == relation)
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

    /// Dispatch the `remember` tool call.
    pub async fn dispatch_remember(&self, args: &str) -> String {
        let input: crate::tool::Remember = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        if input.key.is_empty() {
            return "missing required field: key".to_owned();
        }
        if !self.is_valid_entity(&input.entity_type) {
            return format!(
                "unknown entity_type: '{}'. allowed: {}",
                input.entity_type,
                self.allowed_entities.join(", ")
            );
        }

        let id = entity_id(&input.entity_type, &input.key);
        let text = format!("{} {}", input.key, input.value);
        let vector = match self.embed(&text).await {
            Ok(v) => v,
            Err(e) => return format!("failed to embed entity: {e}"),
        };
        let row = EntityRow {
            id: &id,
            entity_type: &input.entity_type,
            key: &input.key,
            value: &input.value,
            vector,
        };
        match self.lance.upsert_entity(&row).await {
            Ok(()) => format!(
                "remembered [{}] {}: {}",
                input.entity_type, input.key, input.value
            ),
            Err(e) => format!("failed to store entity: {e}"),
        }
    }

    /// Dispatch the `recall` tool call.
    pub async fn dispatch_recall(&self, args: &str) -> String {
        let input: crate::tool::Recall = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        if input.query.is_empty() {
            return "missing required field: query".to_owned();
        }
        let limit = input.limit.unwrap_or(10) as usize;

        match self
            .lance
            .search_entities(&input.query, input.entity_type.as_deref(), limit)
            .await
        {
            Ok(entities) if entities.is_empty() => "no entities found".to_owned(),
            Ok(entities) => entities
                .iter()
                .map(|e| format!("[{}] {}: {}", e.entity_type, e.key, e.value))
                .collect::<Vec<_>>()
                .join("\n"),
            Err(e) => format!("recall failed: {e}"),
        }
    }

    /// Dispatch the `relate` tool call.
    pub async fn dispatch_relate(&self, args: &str) -> String {
        let input: crate::tool::Relate = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        if input.source_key.is_empty() || input.target_key.is_empty() {
            return "missing required field: source_key or target_key".to_owned();
        }
        if input.relation.is_empty() {
            return "missing required field: relation".to_owned();
        }
        if !self.is_valid_relation(&input.relation) {
            return format!(
                "unknown relation: '{}'. allowed: {}",
                input.relation,
                self.allowed_relations.join(", ")
            );
        }

        // Look up source entity.
        let source = match self.lance.find_entity_by_key(&input.source_key).await {
            Ok(Some(e)) => e,
            Ok(None) => return format!("source entity not found: '{}'", input.source_key),
            Err(e) => return format!("failed to look up source: {e}"),
        };

        // Look up target entity.
        let target = match self.lance.find_entity_by_key(&input.target_key).await {
            Ok(Some(e)) => e,
            Ok(None) => return format!("target entity not found: '{}'", input.target_key),
            Err(e) => return format!("failed to look up target: {e}"),
        };

        let row = RelationRow {
            source: &source.id,
            relation: &input.relation,
            target: &target.id,
        };
        match self.lance.upsert_relation(&row).await {
            Ok(()) => format!(
                "related: {} -[{}]-> {}",
                input.source_key, input.relation, input.target_key
            ),
            Err(e) => format!("failed to create relation: {e}"),
        }
    }

    /// Dispatch the `connections` tool call.
    pub async fn dispatch_connections(&self, args: &str) -> String {
        let input: crate::tool::Connections = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        if input.key.is_empty() {
            return "missing required field: key".to_owned();
        }

        // Look up the entity.
        let entity = match self.lance.find_entity_by_key(&input.key).await {
            Ok(Some(e)) => e,
            Ok(None) => return format!("entity not found: '{}'", input.key),
            Err(e) => return format!("failed to look up entity: {e}"),
        };

        let direction = match input.direction.as_deref() {
            Some("incoming") => Direction::Incoming,
            Some("both") => Direction::Both,
            _ => Direction::Outgoing,
        };

        let limit = input
            .limit
            .map(|l| (l as usize).min(100))
            .unwrap_or(self.connection_limit);

        let relations = match self
            .lance
            .find_connections(&entity.id, input.relation.as_deref(), direction, limit)
            .await
        {
            Ok(r) => r,
            Err(e) => return format!("connections query failed: {e}"),
        };

        if relations.is_empty() {
            return "no connections found".to_owned();
        }

        relations
            .iter()
            .map(|r| format!("{} -[{}]-> {}", r.source, r.relation, r.target))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Dispatch the `compact` tool call.
    ///
    /// Returns recent journal context for the compaction flow.
    /// The agent loop detects compact by tool name and triggers compaction.
    pub async fn dispatch_compact(&self, agent: &str) -> String {
        let mut result = String::new();

        // Return recent journal entries for continuity context.
        if let Ok(journals) = self.lance.recent_journals(agent, 3).await
            && !journals.is_empty()
        {
            result.push_str("Previous journal entries:\n");
            for j in &journals {
                let ts = chrono::DateTime::from_timestamp(j.created_at as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| j.created_at.to_string());
                result.push_str(&format!("- [{ts}] {}\n", j.summary));
            }
        }

        result
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

    /// Dispatch the `distill` tool call — semantic search over journal entries.
    pub async fn dispatch_distill(&self, args: &str, agent: &str) -> String {
        let input: crate::tool::Distill = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        if input.query.is_empty() {
            return "missing required field: query".to_owned();
        }
        let limit = input.limit.unwrap_or(5) as usize;

        let vector = match self.embed(&input.query).await {
            Ok(v) => v,
            Err(e) => return format!("failed to embed query: {e}"),
        };

        match self.lance.search_journals(&vector, agent, limit).await {
            Ok(journals) if journals.is_empty() => "no journal entries found".to_owned(),
            Ok(journals) => journals
                .iter()
                .map(|j| {
                    let ts = chrono::DateTime::from_timestamp(j.created_at as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| j.created_at.to_string());
                    format!("[{ts}] {}", j.summary)
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
            Err(e) => format!("distill failed: {e}"),
        }
    }
}

/// Build entity ID: `{entity_type}:{key}`.
fn entity_id(entity_type: &str, key: &str) -> String {
    format!("{entity_type}:{key}")
}

/// Truncate a string at a UTF-8 safe boundary, appending "..." if truncated.
pub fn truncate_utf8(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_owned();
    }
    // Walk backward from max_bytes to find a char boundary.
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

fn merge_defaults(defaults: &[&str], extras: &[String]) -> Vec<String> {
    let mut merged: Vec<String> = defaults.iter().map(|s| (*s).to_owned()).collect();
    for t in extras {
        if !merged.contains(t) {
            merged.push(t.clone());
        }
    }
    merged
}
