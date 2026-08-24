//! Memory entries, one file each.

use crate::backend::{
    self, Backend,
    search::{self, Doc},
};
use anyhow::Result;
use store::{MemoryEntry, interface::Memory};

impl Backend {
    fn memory_path(&self, name: &str) -> std::path::PathBuf {
        self.memory_dir()
            .join(format!("{}.json", backend::encode(name)))
    }
}

impl Memory for Backend {
    async fn memory(&self, name: &str) -> Result<Option<MemoryEntry>> {
        backend::read_json(&self.memory_path(name)).await
    }

    async fn memory_names(&self) -> Result<Vec<String>> {
        backend::names_in(&self.memory_dir(), ".json").await
    }

    async fn memory_names_under(&self, stem: &str) -> Result<Vec<String>> {
        Ok(self
            .memory_names()
            .await?
            .into_iter()
            .filter(|name| name.starts_with(stem))
            .collect())
    }

    async fn put_memory(&self, entry: &MemoryEntry) -> Result<()> {
        backend::write_json(&self.memory_path(&entry.name), entry).await
    }

    async fn remove_memory(&self, name: &str) -> Result<bool> {
        Ok(tokio::fs::remove_file(self.memory_path(name)).await.is_ok())
    }

    /// Aliases are alternative search terms, so they rank as part of the
    /// entry rather than as names of their own.
    async fn search_memory(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let (mut names, mut docs) = (Vec::new(), Vec::new());
        for name in self.memory_names().await? {
            let Some(entry) = self.memory(&name).await? else {
                continue;
            };
            let text = match entry.aliases.is_empty() {
                true => entry.content,
                false => format!("{}\n{}", entry.aliases.join(" "), entry.content),
            };
            names.push(name);
            docs.push(Doc { text, weight: 1.0 });
        }
        Ok(search::rank(&docs, query, limit)
            .into_iter()
            .map(|(at, _)| names[at].clone())
            .collect())
    }
}
