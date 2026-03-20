//! Crabtalk hub package install/uninstall operations.

use crate::{DownloadRegistry, manifest};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::Path;
use wcore::paths::CONFIG_DIR;
use wcore::protocol::message::{
    DownloadCompleted, DownloadCreated, DownloadEvent, DownloadKind, DownloadStep, download_event,
};

/// Remote URL of the crabtalk hub repository.
const CRABTALK_HUB: &str = "https://github.com/crabtalk/hub";

/// Parsed filters for selective install/uninstall.
///
/// When all sets are empty, everything is included (no filtering).
/// Filter format: `"kind:name"` — e.g. `"skill:playwright-cli"`.
struct HubFilter {
    mcps: BTreeSet<String>,
    skills: BTreeSet<String>,
    agents: BTreeSet<String>,
    commands: BTreeSet<String>,
}

impl HubFilter {
    fn parse(filters: &[String]) -> Self {
        let mut f = Self {
            mcps: BTreeSet::new(),
            skills: BTreeSet::new(),
            agents: BTreeSet::new(),
            commands: BTreeSet::new(),
        };
        for raw in filters {
            if let Some((kind, name)) = raw.split_once(':') {
                let set = match kind {
                    "mcp" => &mut f.mcps,
                    "skill" => &mut f.skills,
                    "agent" => &mut f.agents,
                    "command" => &mut f.commands,
                    _ => continue,
                };
                set.insert(name.to_string());
            }
        }
        f
    }

    fn is_empty(&self) -> bool {
        self.mcps.is_empty()
            && self.skills.is_empty()
            && self.agents.is_empty()
            && self.commands.is_empty()
    }

    fn wants_mcp(&self, name: &str) -> bool {
        self.is_empty() || self.mcps.contains(name)
    }

    fn wants_skill(&self, name: &str) -> bool {
        self.is_empty() || self.skills.contains(name)
    }

    fn wants_agent(&self, name: &str) -> bool {
        self.is_empty() || self.agents.contains(name)
    }

    fn wants_command(&self, name: &str) -> bool {
        self.is_empty() || self.commands.contains(name)
    }
}

/// Install a hub package, streaming unified download events.
///
/// Syncs the hub repo, reads the manifest, merges MCP servers
/// into `crab.toml`, copies skill directories into `~/.crabtalk/skills/`,
/// and installs agents (prompt files + their declared skills).
///
/// When `filters` is non-empty, only matching components are installed.
pub fn install(
    package: String,
    registry: std::sync::Arc<tokio::sync::Mutex<DownloadRegistry>>,
    filters: Vec<String>,
) -> impl futures_core::Stream<Item = Result<DownloadEvent>> {
    async_stream::try_stream! {
        let id = registry
            .lock()
            .await
            .start(DownloadKind::Hub, package.to_string());
        let event = DownloadEvent {
            event: Some(download_event::Event::Created(DownloadCreated {
                id,
                kind: DownloadKind::Hub as i32,
                label: package.to_string(),
            })),
        };
        yield event.clone();
        registry.lock().await.broadcast(event);

        let filter = HubFilter::parse(&filters);

        // Sync hub repo (clone or update).
        let hub_dir = CONFIG_DIR.join("hub");
        git_sync(CRABTALK_HUB, &hub_dir).await.context("failed to sync hub repo")?;

        let (scope, name) = parse_package(&package)?;
        let manifest = read_manifest(scope, name)?;

        // Merge MCP servers.
        let wanted_mcps: Vec<_> = manifest
            .mcps
            .iter()
            .filter(|(k, _)| filter.wants_mcp(k.as_str()))
            .collect();
        if !wanted_mcps.is_empty() {
            let event = step_event(id, "adding MCP servers…");
            yield event.clone();
            registry.lock().await.broadcast(event);
            merge_section_filtered("mcps", &wanted_mcps)?;
        }

        // Collect skill keys from wanted agents.
        let wanted_agents: Vec<_> = manifest
            .agents
            .iter()
            .filter(|(k, _)| filter.wants_agent(k.as_str()))
            .collect();
        let mut agent_skill_keys: BTreeSet<&String> = BTreeSet::new();
        for (_, agent) in &wanted_agents {
            for sk in &agent.skills {
                agent_skill_keys.insert(sk);
            }
        }

        // Install skills (top-level wanted + agent-referenced).
        let all_skill_keys: BTreeSet<&String> = manifest
            .skills
            .keys()
            .filter(|k| filter.wants_skill(k.as_str()))
            .chain(agent_skill_keys.iter().copied())
            .collect();

        if !all_skill_keys.is_empty() {
            let event = step_event(id, "installing skills…");
            yield event.clone();
            registry.lock().await.broadcast(event);

            // Clone the source repo once, then copy per-skill subdirectories.
            let slug = repo_slug(&manifest.package.repository);
            if slug.is_empty() {
                Err(anyhow::anyhow!("manifest has no repository URL for skill install"))?;
            }
            let repo_dir = CONFIG_DIR.join(".cache").join("repos").join(&slug);
            std::fs::create_dir_all(repo_dir.parent().context("repo cache path has no parent")?)
                .context("failed to create repo cache dir")?;
            git_sync(&manifest.package.repository, &repo_dir)
                .await
                .with_context(|| format!("failed to sync repo {}", &manifest.package.repository))?;

            let skills_dir = CONFIG_DIR.join("skills");
            std::fs::create_dir_all(&skills_dir).context("failed to create skills dir")?;

            for key in &all_skill_keys {
                let skill = manifest.skills.get(*key).ok_or_else(|| {
                    anyhow::anyhow!("agent references skill '{key}' not found in [skills]")
                })?;
                let event = step_event(id, &format!("installing skill {key}…"));
                yield event.clone();
                registry.lock().await.broadcast(event);

                let src = repo_dir.join(skill.path.as_str());
                let dst = skills_dir.join(key.as_str());
                if dst.exists() {
                    std::fs::remove_dir_all(&dst)
                        .with_context(|| format!("failed to remove old skill {key}"))?;
                }
                copy_dir_all(&src, &dst)
                    .with_context(|| format!("failed to copy skill {key}"))?;
            }
        }

        // Install agents (copy prompt .md files).
        if !wanted_agents.is_empty() {
            let event = step_event(id, "installing agents…");
            yield event.clone();
            registry.lock().await.broadcast(event);
            install_agents_filtered(scope, &wanted_agents)?;
        }

        // Register commands (merge metadata into crab.toml).
        let wanted_cmds: Vec<_> = manifest
            .commands
            .iter()
            .filter(|(k, _)| filter.wants_command(k.as_str()))
            .collect();
        if !wanted_cmds.is_empty() {
            let event = step_event(id, "registering commands…");
            yield event.clone();
            registry.lock().await.broadcast(event);
            merge_section_filtered("commands", &wanted_cmds)?;
        }

        registry.lock().await.complete(id);
        let event = DownloadEvent {
            event: Some(download_event::Event::Completed(DownloadCompleted { id })),
        };
        yield event.clone();
        registry.lock().await.broadcast(event);
    }
}

/// Uninstall a hub package, streaming unified download events.
///
/// Reads the manifest from the local hub repo (no network sync), removes MCP
/// servers from `crab.toml`, deletes skill directories and agent prompt files.
///
/// When `filters` is non-empty, only matching components are removed.
pub fn uninstall(
    package: String,
    registry: std::sync::Arc<tokio::sync::Mutex<DownloadRegistry>>,
    filters: Vec<String>,
) -> impl futures_core::Stream<Item = Result<DownloadEvent>> {
    async_stream::try_stream! {
        let id = registry
            .lock()
            .await
            .start(DownloadKind::Hub, package.to_string());
        let event = DownloadEvent {
            event: Some(download_event::Event::Created(DownloadCreated {
                id,
                kind: DownloadKind::Hub as i32,
                label: package.to_string(),
            })),
        };
        yield event.clone();
        registry.lock().await.broadcast(event);

        let filter = HubFilter::parse(&filters);

        let (scope, name) = parse_package(&package)?;
        let manifest = read_manifest(scope, name)?;

        let mcp_keys: Vec<_> = manifest
            .mcps
            .keys()
            .filter(|k| filter.wants_mcp(k.as_str()))
            .collect();
        if !mcp_keys.is_empty() {
            let event = step_event(id, "removing MCP servers…");
            yield event.clone();
            registry.lock().await.broadcast(event);
            remove_keys_from_section("mcps", &mcp_keys)?;
        }

        // Collect agent-declared skill keys (mirrors install logic).
        let wanted_agents: Vec<_> = manifest
            .agents
            .iter()
            .filter(|(k, _)| filter.wants_agent(k.as_str()))
            .collect();
        let mut agent_skill_keys: BTreeSet<&String> = BTreeSet::new();
        for (_, agent) in &wanted_agents {
            for sk in &agent.skills {
                agent_skill_keys.insert(sk);
            }
        }

        let skill_keys: BTreeSet<&String> = manifest
            .skills
            .keys()
            .filter(|k| filter.wants_skill(k.as_str()))
            .chain(agent_skill_keys.iter().copied())
            .collect();
        if !skill_keys.is_empty() {
            let event = step_event(id, "removing skills…");
            yield event.clone();
            registry.lock().await.broadcast(event);
            let skills_dir = CONFIG_DIR.join("skills");
            for key in &skill_keys {
                let dst = skills_dir.join(key.as_str());
                if dst.exists() {
                    std::fs::remove_dir_all(&dst)
                        .with_context(|| format!("failed to remove skill {key}"))?;
                }
            }
        }

        if !wanted_agents.is_empty() {
            let event = step_event(id, "removing agents…");
            yield event.clone();
            registry.lock().await.broadcast(event);
            let agents_dir = CONFIG_DIR.join(wcore::paths::AGENTS_DIR);
            for (name, _) in &wanted_agents {
                let dst = agents_dir.join(format!("{name}.md"));
                if dst.exists() {
                    std::fs::remove_file(&dst)
                        .with_context(|| format!("failed to remove agent prompt {}", dst.display()))?;
                }
            }
        }

        let cmd_keys: Vec<_> = manifest
            .commands
            .keys()
            .filter(|k| filter.wants_command(k.as_str()))
            .collect();
        if !cmd_keys.is_empty() {
            let event = step_event(id, "removing commands…");
            yield event.clone();
            registry.lock().await.broadcast(event);
            remove_keys_from_section("commands", &cmd_keys)?;
        }

        registry.lock().await.complete(id);
        let event = DownloadEvent {
            event: Some(download_event::Event::Completed(DownloadCompleted { id })),
        };
        yield event.clone();
        registry.lock().await.broadcast(event);
    }
}

// ── Helpers ───────────────────────────────────────────────────────

/// Ensure `dest` is a shallow clone of `url`, creating or updating as needed.
async fn git_sync(url: &str, dest: &Path) -> Result<()> {
    use tokio::process::Command;

    if dest.exists() {
        let status = Command::new("git")
            .args([
                "-C",
                &dest.to_string_lossy(),
                "fetch",
                "--depth=1",
                "origin",
            ])
            .status()
            .await
            .context("git fetch failed")?;
        anyhow::ensure!(status.success(), "git fetch exited with {status}");

        let status = Command::new("git")
            .args([
                "-C",
                &dest.to_string_lossy(),
                "reset",
                "--hard",
                "origin/HEAD",
            ])
            .status()
            .await
            .context("git reset failed")?;
        anyhow::ensure!(status.success(), "git reset exited with {status}");
    } else {
        let status = Command::new("git")
            .args(["clone", "--depth=1", url, &dest.to_string_lossy()])
            .status()
            .await
            .context("git clone failed")?;
        anyhow::ensure!(status.success(), "git clone exited with {status}");
    }
    Ok(())
}

/// Parse a `scope/name` package string into `(scope, name)`.
fn parse_package(package: &str) -> Result<(&str, &str)> {
    let mut parts = package.splitn(2, '/');
    let scope = parts.next().filter(|s| !s.is_empty());
    let name = parts.next().filter(|s| !s.is_empty());
    match (scope, name) {
        (Some(s), Some(n)) => Ok((s, n)),
        _ => anyhow::bail!("package must be in `scope/name` format, got: {package}"),
    }
}

/// Read and deserialize the manifest for a package from the local hub repo.
fn read_manifest(scope: &str, name: &str) -> Result<manifest::Manifest> {
    let hub_dir = CONFIG_DIR.join("hub");
    let path = hub_dir.join(scope).join(format!("{name}.toml"));
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read manifest at {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("invalid manifest at {}", path.display()))
}

/// Merge serializable entries into a named section of `crab.toml`.
fn merge_section_filtered<T: serde::Serialize>(
    section: &str,
    entries: &[(&String, &T)],
) -> Result<()> {
    use toml_edit::DocumentMut;

    let config_path = CONFIG_DIR.join("crab.toml");
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;
    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("invalid TOML in {}", config_path.display()))?;

    let table = doc
        .entry(section)
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .with_context(|| format!("{section} is not a table"))?;

    for (key, cfg) in entries {
        let doc = toml_edit::ser::to_document(cfg)
            .with_context(|| format!("failed to serialize entry for {key}"))?;
        let item = toml_edit::Item::Table(doc.as_table().clone());
        table.insert(key.as_str(), item);
    }

    std::fs::write(&config_path, doc.to_string())
        .with_context(|| format!("failed to write {}", config_path.display()))?;
    Ok(())
}

/// Remove keys from a TOML section in `crab.toml`.
fn remove_keys_from_section(section: &str, keys: &[&String]) -> Result<()> {
    use toml_edit::DocumentMut;

    let config_path = CONFIG_DIR.join("crab.toml");
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;
    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("invalid TOML in {}", config_path.display()))?;

    if let Some(table) = doc.get_mut(section).and_then(|v| v.as_table_mut()) {
        for key in keys {
            table.remove(key.as_str());
        }
    }

    std::fs::write(&config_path, doc.to_string())
        .with_context(|| format!("failed to write {}", config_path.display()))?;
    Ok(())
}

/// Install selected agent prompt files from the hub repo.
fn install_agents_filtered(
    scope: &str,
    agents: &[(&String, &manifest::AgentResource)],
) -> Result<()> {
    let agents_dir = CONFIG_DIR.join(wcore::paths::AGENTS_DIR);
    std::fs::create_dir_all(&agents_dir).context("failed to create agents directory")?;

    let hub_dir = CONFIG_DIR.join("hub");
    for (name, agent) in agents {
        let src = hub_dir.join(scope).join(agent.prompt.as_str());
        let dst = agents_dir.join(format!("{name}.md"));
        std::fs::copy(&src, &dst).with_context(|| {
            format!(
                "failed to copy agent prompt {} -> {}",
                src.display(),
                dst.display()
            )
        })?;
    }
    Ok(())
}

/// Build a step event.
fn step_event(id: u64, message: &str) -> DownloadEvent {
    DownloadEvent {
        event: Some(download_event::Event::Step(DownloadStep {
            id,
            message: message.to_string(),
        })),
    }
}

/// Convert a repo URL to a filesystem-safe slug.
///
/// e.g. `https://github.com/microsoft/playwright-cli` → `github-com-microsoft-playwright-cli`
fn repo_slug(url: &str) -> String {
    url.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Recursively copy `src` directory into `dst`.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in
        std::fs::read_dir(src).with_context(|| format!("cannot read dir {}", src.display()))?
    {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to)
                .with_context(|| format!("failed to copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}
