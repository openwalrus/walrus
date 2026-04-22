//! Filesystem-backed persistence.
//!
//! [`FsStorage`] implements [`Storage`](wcore::storage::Storage)
//! with TOML configs, markdown prompts, and JSON session files.

pub use self::fs::FsStorage;
pub use loader::{DEFAULT_CONFIG, scaffold_config_dir};
use std::{
    fs as stdfs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use wcore::AgentConfig;

mod fs;
mod loader;

/// Built-in crab agent prompt (from `prompts/crab.md`).
pub const CRAB_PROMPT: &str = crate::hooks::memory::DEFAULT_SOUL;

/// Construct the default `crab` system agent.
///
/// Used by [`FsStorage::scaffold`] to seed a fresh install and by the
/// daemon as a fallback when no `crab` agent is stored. The model is
/// left unset so the registry's active model is used.
pub fn default_crab() -> AgentConfig {
    let mut cfg = AgentConfig::new(wcore::paths::DEFAULT_AGENT);
    cfg.system_prompt = CRAB_PROMPT.to_owned();
    cfg
}

/// Atomic write: same-directory tmp file + rename.
pub fn atomic_write(path: &Path, data: &[u8]) -> anyhow::Result<()> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut tmp_os = path.to_path_buf().into_os_string();
    tmp_os.push(format!(".tmp.{}.{}", std::process::id(), nanos));
    let tmp_path = PathBuf::from(tmp_os);
    stdfs::write(&tmp_path, data)?;
    if let Err(e) = stdfs::rename(&tmp_path, path) {
        let _ = stdfs::remove_file(&tmp_path);
        return Err(e.into());
    }
    Ok(())
}
