use crate::{
    bm25::{Index, tokenize},
    entry::{Entry, EntryId, EntryKind},
    error::{Error, Result},
    op::Op,
};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

/// In-RAM memory connection. Persistence, dump/load, and file-path
/// semantics land in later phases.
pub struct Memory {
    entries: HashMap<EntryId, Entry>,
    by_name: HashMap<String, EntryId>,
    index: Index,
    next_id: EntryId,
}

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub entry: Entry,
    pub score: f64,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            by_name: HashMap::new(),
            index: Index::new(),
            next_id: 1,
        }
    }

    pub fn apply(&mut self, op: Op) -> Result<()> {
        match op {
            Op::Add {
                name,
                content,
                aliases,
                kind,
            } => self.add(name, content, aliases, kind),
            Op::Update {
                name,
                content,
                aliases,
            } => self.update(&name, content, aliases),
            Op::Alias { name, aliases } => self.set_aliases(&name, aliases),
            Op::Remove { name } => self.remove(&name),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.by_name.get(name).and_then(|id| self.entries.get(id))
    }

    pub fn list(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values()
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        self.index
            .search(query, limit)
            .into_iter()
            .filter_map(|(id, score)| {
                self.entries.get(&id).map(|e| SearchHit {
                    entry: e.clone(),
                    score,
                })
            })
            .collect()
    }

    fn add(
        &mut self,
        name: String,
        content: String,
        aliases: Vec<String>,
        kind: EntryKind,
    ) -> Result<()> {
        if self.by_name.contains_key(&name) {
            return Err(Error::Duplicate(name));
        }
        let id = self.next_id;
        self.next_id += 1;
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let entry = Entry {
            id,
            name: name.clone(),
            content,
            aliases,
            created_at,
            kind,
        };
        self.reindex(&entry);
        self.by_name.insert(name, id);
        self.entries.insert(id, entry);
        Ok(())
    }

    fn update(&mut self, name: &str, content: String, aliases: Vec<String>) -> Result<()> {
        let id = *self
            .by_name
            .get(name)
            .ok_or_else(|| Error::NotFound(name.to_owned()))?;
        let entry = self.entries.get_mut(&id).expect("entry id out of sync");
        entry.content = content;
        entry.aliases = aliases;
        let snapshot = entry.clone();
        self.reindex(&snapshot);
        Ok(())
    }

    fn set_aliases(&mut self, name: &str, aliases: Vec<String>) -> Result<()> {
        let id = *self
            .by_name
            .get(name)
            .ok_or_else(|| Error::NotFound(name.to_owned()))?;
        let entry = self.entries.get_mut(&id).expect("entry id out of sync");
        entry.aliases = aliases;
        let snapshot = entry.clone();
        self.reindex(&snapshot);
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<()> {
        let id = self
            .by_name
            .remove(name)
            .ok_or_else(|| Error::NotFound(name.to_owned()))?;
        self.entries.remove(&id);
        self.index.remove(id);
        Ok(())
    }

    fn reindex(&mut self, entry: &Entry) {
        let mut terms = tokenize(&entry.content);
        for alias in &entry.aliases {
            terms.extend(tokenize(alias));
        }
        self.index.insert(entry.id, &terms);
    }
}
