//! `crabtalk gateway` — forwards to `crabtalk-gateway` binary (cargo-style).

use anyhow::Result;
use clap::Args;

/// Forward to the crabtalk-gateway binary.
#[derive(Args, Debug)]
pub struct Gateway {
    /// Arguments passed through to `crabtalk-gateway`.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl Gateway {
    pub fn run(self) -> Result<()> {
        let binary = find_gateway_binary()?;
        let status = std::process::Command::new(&binary)
            .args(&self.args)
            .status()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    }
}

fn find_gateway_binary() -> Result<std::path::PathBuf> {
    // Look next to the current binary first (same install dir).
    if let Ok(current) = std::env::current_exe()
        && let Some(dir) = current.parent()
    {
        let candidate = dir.join("crabtalk-gateway");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // Fall back to PATH lookup.
    let path = std::env::var("PATH").unwrap_or_default();
    for dir in path.split(':') {
        let candidate = std::path::PathBuf::from(dir).join("crabtalk-gateway");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "crabtalk-gateway not found. Install with: cargo install --path crates/gateway --features cli"
    )
}
