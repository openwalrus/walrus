//! `crabtalk daemon logs` — view daemon logs.
//!
//! Thin wrapper around `tail` — all extra flags are passed through.

use anyhow::{Context, Result};
use wcore::paths::LOGS_DIR;

/// Display daemon log output by delegating to `tail`.
pub fn logs(tail_args: &[String]) -> Result<()> {
    let path = LOGS_DIR.join("daemon.log");
    if !path.exists() {
        anyhow::bail!("log file not found: {}", path.display());
    }

    let status = std::process::Command::new("tail")
        .args(tail_args)
        .arg(&path)
        .status()
        .context("failed to run tail")?;
    if !status.success() {
        anyhow::bail!("tail exited with {}", status);
    }
    Ok(())
}
