//! Hub package management command.

use crate::repl::runner::Runner;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use crabhub::manifest::Manifest;
use dialoguer::{MultiSelect, theme::ColorfulTheme};
use std::path::{Path, PathBuf};
use wcore::paths::CONFIG_DIR;

/// Manage hub packages.
#[derive(Args, Debug)]
pub struct Hub {
    /// Hub subcommand.
    #[command(subcommand)]
    pub command: HubCommand,
}

/// Hub subcommands.
#[derive(Subcommand, Debug)]
pub enum HubCommand {
    /// Install a hub package.
    Install(HubPackage),
    /// Uninstall a hub package.
    Uninstall(HubPackage),
    /// Test manifest parsing for all .toml files in a hub directory.
    Test(HubTest),
}

/// Arguments for the test subcommand.
#[derive(Args, Debug)]
pub struct HubTest {
    /// Path to a manifest .toml file to validate.
    pub path: PathBuf,
}

/// Package argument shared by install and uninstall.
#[derive(Args, Debug)]
pub struct HubPackage {
    /// Package identifier in `scope/name` format.
    pub package: String,

    /// Install only specific skills by name.
    #[arg(long)]
    pub skill: Vec<String>,

    /// Install only specific MCP servers by name.
    #[arg(long)]
    pub mcp: Vec<String>,

    /// Install only specific agents by name.
    #[arg(long)]
    pub agent: Vec<String>,

    /// Install only specific commands by name.
    #[arg(long)]
    pub command: Vec<String>,
}

impl HubPackage {
    /// Build filter strings from the CLI flags.
    fn filters(&self) -> Vec<String> {
        let mut out = Vec::new();
        for s in &self.skill {
            out.push(format!("skill:{s}"));
        }
        for s in &self.mcp {
            out.push(format!("mcp:{s}"));
        }
        for s in &self.agent {
            out.push(format!("agent:{s}"));
        }
        for s in &self.command {
            out.push(format!("command:{s}"));
        }
        out
    }

    fn has_filters(&self) -> bool {
        !self.skill.is_empty()
            || !self.mcp.is_empty()
            || !self.agent.is_empty()
            || !self.command.is_empty()
    }
}

impl Hub {
    /// Run the hub command.
    pub async fn run(self, runner: &mut Runner) -> Result<()> {
        if let HubCommand::Test(t) = self.command {
            return test_manifest(&t.path);
        }

        let (pkg, is_install, has_flags, explicit_filters) = match self.command {
            HubCommand::Install(p) => {
                let has = p.has_filters();
                let filters = p.filters();
                (p.package, true, has, filters)
            }
            HubCommand::Uninstall(p) => {
                let has = p.has_filters();
                let filters = p.filters();
                (p.package, false, has, filters)
            }
            HubCommand::Test(_) => unreachable!(),
        };

        let on_step = |msg: &str| println!("  {msg}");

        if is_install {
            // Sync hub repo and read manifest for picker + setup.
            let (scope, name) = crabhub::package::parse_package(&pkg)?;
            let hub_dir = CONFIG_DIR.join("hub");
            let hub_url = "https://github.com/aspect-build/crabtalk-hub";
            crabhub::package::git_sync(hub_url, &hub_dir).await?;
            let manifest = crabhub::package::read_manifest(scope, name)?;

            // Determine filters: explicit flags bypass picker.
            let filters = if has_flags {
                explicit_filters
            } else {
                pick_components(&manifest)?
            };

            crabhub::package::install(&pkg, &filters, on_step).await?;
            println!("Done: {pkg}");

            // Reload daemon to pick up new components.
            let _ = runner.reload().await;
            println!("Daemon reloaded.");
            println!("\nConfigure env vars in config.toml [env] section if needed.");
        } else {
            crabhub::package::uninstall(&pkg, &explicit_filters, on_step).await?;
            println!("Done: {pkg}");
        }

        Ok(())
    }
}

/// Show an interactive component picker. Returns filter strings.
/// If only one component exists, skips the picker and returns empty (install all).
fn pick_components(manifest: &Manifest) -> Result<Vec<String>> {
    let mut items: Vec<String> = Vec::new();
    for key in manifest.skills.keys() {
        items.push(format!("skill:{key}"));
    }
    for key in manifest.mcps.keys() {
        items.push(format!("mcp:{key}"));
    }
    for key in manifest.agents.keys() {
        items.push(format!("agent:{key}"));
    }
    for key in manifest.commands.keys() {
        items.push(format!("command:{key}"));
    }

    // Single component or empty — install everything, no picker needed.
    if items.len() <= 1 {
        return Ok(vec![]);
    }

    let defaults: Vec<bool> = vec![true; items.len()];
    let theme = ColorfulTheme::default();
    let selections = MultiSelect::with_theme(&theme)
        .with_prompt("Select components to install")
        .items(&items)
        .defaults(&defaults)
        .interact()?;

    // All selected — no filter needed.
    if selections.len() == items.len() {
        return Ok(vec![]);
    }

    Ok(selections.into_iter().map(|i| items[i].clone()).collect())
}

/// Parse a single manifest .toml and report success or the parse error.
fn test_manifest(path: &Path) -> Result<()> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let manifest: Manifest =
        toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;
    println!("ok  {}", manifest.package.name);
    Ok(())
}
