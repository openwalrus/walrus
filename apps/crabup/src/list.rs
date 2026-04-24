//! Read `~/.cargo/.crates.toml` and surface installed crabtalk crates.

use anyhow::{Context, Result};

use crate::registry;

/// Return the set of installed crabtalk-owned crates, sorted.
pub fn installed() -> Result<Vec<String>> {
    let path = dirs::home_dir()
        .context("could not resolve home directory")?
        .join(".cargo/.crates.toml");
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let parsed: toml::Value =
        toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;
    let Some(v1) = parsed.get("v1").and_then(|v| v.as_table()) else {
        return Ok(vec![]);
    };

    let mut names: Vec<String> = v1
        .keys()
        .filter_map(|k| {
            let krate = k.split_whitespace().next()?;
            registry::is_crabtalk(krate).then(|| krate.to_string())
        })
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}
