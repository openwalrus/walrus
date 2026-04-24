//! Thin wrappers over `cargo install` / `cargo uninstall`.

use anyhow::{Context, Result, bail};
use std::process::Command;

pub fn install(krate: &str, version: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(["install", krate]);
    if let Some(v) = version {
        cmd.args(["--version", v]);
    }
    let status = cmd
        .status()
        .context("failed to run `cargo` — install Rust from https://rustup.rs")?;
    if !status.success() {
        bail!("cargo install {krate} failed");
    }
    Ok(())
}

pub fn uninstall(krate: &str) -> Result<()> {
    let status = Command::new("cargo")
        .args(["uninstall", krate])
        .status()
        .context("failed to run `cargo` — install Rust from https://rustup.rs")?;
    if !status.success() {
        bail!("cargo uninstall {krate} failed");
    }
    Ok(())
}
