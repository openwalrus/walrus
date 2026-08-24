//! Crab's store: the six interfaces over a directory.
//!
//! No key-value primitive underneath. A session is a directory of JSONL,
//! memory and skills are files, and that partition is what lets two
//! `crab` processes write at once — they never hold the same file.
//!
//! A name arrives from a person or a model and has to become a filename,
//! so anything outside `[A-Za-z0-9._-]` is percent-encoded. Two names
//! differing only in case still collide on a case-insensitive
//! filesystem; that is the filesystem's property, not one this encoding
//! tries to hide.

use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

mod agent;
mod event;
mod harness;
mod memory;
mod search;
mod session;
mod skill;

/// One crab install's state, rooted at a directory.
pub struct Backend {
    pub root: PathBuf,
}

impl Backend {
    /// Open a store, creating the directories it writes into.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        for dir in ["agents", "sessions", "memory", "skills", "harnesses"] {
            std::fs::create_dir_all(root.join(dir))?;
        }
        Ok(Self { root })
    }

    pub(crate) fn agents_dir(&self) -> PathBuf {
        self.root.join("agents")
    }

    pub(crate) fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    pub(crate) fn memory_dir(&self) -> PathBuf {
        self.root.join("memory")
    }

    pub(crate) fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    pub(crate) fn harnesses_dir(&self) -> PathBuf {
        self.root.join("harnesses")
    }
}

/// Filename-safe form of a name, reversed by [`decode`].
pub(crate) fn encode(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for byte in name.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The name a filename was encoded from. A filename this never produced
/// decodes to itself rather than erroring — a directory can hold files
/// nothing here wrote.
pub(crate) fn decode(name: &str) -> String {
    let bytes = name.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' && at + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[at + 1..at + 3]).unwrap_or_default();
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                at += 3;
                continue;
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| name.to_owned())
}

/// Names in a directory, decoded, with `suffix` stripped. Ascending, and
/// empty when the directory does not exist.
pub(crate) async fn names_in(dir: &Path, suffix: &str) -> Result<Vec<String>> {
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(suffix) else {
            continue;
        };
        out.push(decode(stem));
    }
    out.sort();
    Ok(out)
}

pub(crate) async fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    let Ok(bytes) = tokio::fs::read(path).await else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}

/// Write through a temporary file, so a reader never sees half of one and
/// two writers race to a whole document rather than into each other.
pub(crate) async fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, serde_json::to_vec_pretty(value)?).await?;
    Ok(tokio::fs::rename(&tmp, path).await?)
}

/// Append one JSON document as a line. Every writer of a given file is
/// the one process that owns the session, so an append needs no lock.
pub(crate) async fn append_json<T: Serialize>(path: &Path, values: &[T]) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    if values.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut buf = Vec::new();
    for value in values {
        buf.extend_from_slice(&serde_json::to_vec(value)?);
        buf.push(b'\n');
    }
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    Ok(file.write_all(&buf).await?)
}

/// Every JSON line of a file. A torn final line — a crash mid-append —
/// is dropped, the way a torn record ends a log replay.
pub(crate) async fn read_lines<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let Ok(bytes) = tokio::fs::read(path).await else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for line in bytes.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        match serde_json::from_slice(line) {
            Ok(value) => out.push(value),
            Err(e) => tracing::warn!("{}: discarding unreadable line: {e}", path.display()),
        }
    }
    Ok(out)
}

/// Rewrite a file's lines. Used where an append cannot express the change
/// — truncation, and dropping one entry from a set.
pub(crate) async fn write_lines<T: Serialize>(path: &Path, values: &[T]) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut buf = Vec::new();
    for value in values {
        buf.extend_from_slice(&serde_json::to_vec(value)?);
        buf.push(b'\n');
    }
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, buf).await?;
    Ok(tokio::fs::rename(&tmp, path).await?)
}
