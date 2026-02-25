//! Memory trait implementation for SqliteMemory.

use agent::{Embedder, Memory, MemoryEntry, RecallOptions};
use anyhow::Result;
use crate::sql;
use crate::utils::now_unix;
use crate::SqliteMemory;
use std::future::Future;

impl<E: Embedder> Memory for SqliteMemory<E> {
    fn get(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        let now = now_unix();
        conn.execute(sql::TOUCH_ACCESS, rusqlite::params![now as i64, key])
            .ok();
        conn.query_row(sql::SELECT_VALUE, [key], |row| row.get(0))
            .ok()
    }

    fn entries(&self) -> Vec<(String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql::SELECT_ENTRIES).unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    fn set(&self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        let key = key.into();
        let value = value.into();
        let conn = self.conn.lock().unwrap();
        let now = now_unix() as i64;

        let old: Option<String> = conn
            .query_row(sql::SELECT_VALUE, [&key], |row| row.get(0))
            .ok();

        conn.execute(sql::UPSERT, rusqlite::params![key, value, now])
            .ok();

        old
    }

    fn remove(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        let old: Option<String> = conn
            .query_row(sql::SELECT_VALUE, [key], |row| row.get(0))
            .ok();
        if old.is_some() {
            conn.execute(sql::DELETE, [key]).ok();
        }
        old
    }

    fn store(
        &self,
        key: impl Into<String> + Send,
        value: impl Into<String> + Send,
    ) -> impl Future<Output = Result<()>> + Send {
        let key = key.into();
        let value = value.into();

        let conn = self.conn.lock().unwrap();
        let now = now_unix() as i64;

        conn.execute(sql::UPSERT, rusqlite::params![key, value, now])
            .ok();

        async { Ok(()) }
    }

    fn recall(
        &self,
        query: &str,
        options: RecallOptions,
    ) -> impl Future<Output = Result<Vec<MemoryEntry>>> + Send {
        let result = self.recall_sync(query, &options);
        async move { result }
    }

    fn compile_relevant(&self, query: &str) -> impl Future<Output = String> + Send {
        let opts = RecallOptions {
            limit: 5,
            ..Default::default()
        };
        let entries = self.recall_sync(query, &opts).unwrap_or_default();
        let compiled = if entries.is_empty() {
            String::new()
        } else {
            let mut out = String::from("<memory>\n");
            for entry in &entries {
                out.push_str(&format!("<{}>\n", entry.key));
                out.push_str(&entry.value);
                if !entry.value.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str(&format!("</{}>\n", entry.key));
            }
            out.push_str("</memory>");
            out
        };
        async move { compiled }
    }
}
