//! Configuration loading and first-run scaffolding.
//!
//! Handles filesystem I/O: reads agent prompt directories, scaffolds the
//! config directory structure on first run, migrates from old layouts, and
//! resolves package manifests into a merged in-memory view.

use crate::hook::mcp::McpServerConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use wcore::{
    AgentConfig,
    paths::{AGENTS_DIR, CONFIG_FILE, LOCAL_DIR, PACKAGES_DIR, SKILLS_DIR},
};

/// Default configuration template, embedded from the checked-in `config.toml`.
pub const DEFAULT_CONFIG: &str = include_str!("../../config.toml");

// ── Manifest types ──────────────────────────────────────────────────

/// Package manifest format shared by `local/CrabTalk.toml` and
/// `packages/scope/name.toml`. Same shape as hub manifests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManifestConfig {
    /// Package metadata (optional for local manifest).
    #[serde(default)]
    pub package: Option<PackageMeta>,
    /// MCP server configurations.
    #[serde(default)]
    pub mcps: BTreeMap<String, McpServerConfig>,
    /// Per-agent configurations (name → config).
    #[serde(default)]
    pub agents: BTreeMap<String, AgentConfig>,
}

/// Package metadata in a manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageMeta {
    /// Package name.
    #[serde(default)]
    pub name: String,
    /// Source repository URL.
    #[serde(default)]
    pub repository: String,
}

impl ManifestConfig {
    /// Load a manifest from a TOML file. Returns `None` if file doesn't exist.
    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read manifest at {}", path.display()))?;
        let manifest: Self = toml::from_str(&content)
            .with_context(|| format!("invalid manifest at {}", path.display()))?;
        Ok(Some(manifest))
    }
}

// ── Resolved manifest ───────────────────────────────────────────────

/// Merged view of all manifests (local + packages). Built at daemon startup.
#[derive(Debug, Default)]
pub struct ResolvedManifest {
    /// All MCP server configs (local wins over packages).
    pub mcps: BTreeMap<String, McpServerConfig>,
    /// All agent configs from manifests (local wins over packages).
    pub agents: BTreeMap<String, AgentConfig>,
    /// Skill directories to scan (local first, then packages).
    pub skill_dirs: Vec<PathBuf>,
    /// Agent directories to scan (local first, then packages).
    pub agent_dirs: Vec<PathBuf>,
}

/// Resolve all manifests into a single merged view.
///
/// Loads `local/CrabTalk.toml` first (highest priority), then scans
/// `packages/*/*.toml`. For each package with a `repository` field, resolves
/// skill and agent directories from `.cache/repos/{slug}/`. Local always wins
/// on name conflicts; between packages, first alphabetically wins.
pub fn resolve_manifests(config_dir: &Path) -> ResolvedManifest {
    let mut resolved = ResolvedManifest::default();

    // Local dirs always come first.
    let local_skills = config_dir.join(SKILLS_DIR);
    if local_skills.exists() {
        resolved.skill_dirs.push(local_skills);
    }
    let local_agents = config_dir.join(AGENTS_DIR);
    if local_agents.exists() {
        resolved.agent_dirs.push(local_agents);
    }

    // Load local manifest.
    let local_manifest_path = config_dir.join(LOCAL_DIR).join("CrabTalk.toml");
    if let Ok(Some(manifest)) = ManifestConfig::load(&local_manifest_path) {
        merge_manifest(&mut resolved, &manifest, "local");
    }

    // Scan packages/*/*.toml (sorted for deterministic order).
    let packages_dir = config_dir.join(PACKAGES_DIR);
    if let Ok(scopes) = std::fs::read_dir(&packages_dir) {
        let mut scope_entries: Vec<_> = scopes.flatten().collect();
        scope_entries.sort_by_key(|e| e.file_name());

        for scope_entry in scope_entries {
            let scope_path = scope_entry.path();
            if !scope_path.is_dir() {
                // Also handle scope-level .toml files (flat packages/*/*.toml).
                if scope_path.extension().is_some_and(|e| e == "toml") {
                    load_package_manifest(config_dir, &scope_path, &mut resolved);
                }
                continue;
            }
            if let Ok(packages) = std::fs::read_dir(&scope_path) {
                let mut pkg_entries: Vec<_> = packages.flatten().collect();
                pkg_entries.sort_by_key(|e| e.file_name());

                for pkg_entry in pkg_entries {
                    let pkg_path = pkg_entry.path();
                    if pkg_path.extension().is_some_and(|e| e == "toml") {
                        load_package_manifest(config_dir, &pkg_path, &mut resolved);
                    }
                }
            }
        }
    }

    resolved
}

/// Load a single package manifest and merge it into the resolved state.
fn load_package_manifest(config_dir: &Path, path: &Path, resolved: &mut ResolvedManifest) {
    let source = path
        .strip_prefix(config_dir.join(PACKAGES_DIR))
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();

    let manifest = match ManifestConfig::load(path) {
        Ok(Some(m)) => m,
        Ok(None) => return,
        Err(e) => {
            tracing::warn!("failed to load package manifest {}: {e}", path.display());
            return;
        }
    };

    // Resolve cached repo dirs for skills/agents.
    if let Some(ref pkg) = manifest.package
        && !pkg.repository.is_empty()
    {
        let slug = repo_slug(&pkg.repository);
        let repo_dir = config_dir.join(".cache").join("repos").join(&slug);
        if repo_dir.exists() {
            let skills = repo_dir.join("skills");
            if skills.exists() && skills.is_dir() {
                resolved.skill_dirs.push(skills);
            }
            let agents = repo_dir.join("agents");
            if agents.exists() && agents.is_dir() {
                resolved.agent_dirs.push(agents);
            }
        }
    }

    merge_manifest(resolved, &manifest, &source);
}

/// Merge a manifest's MCPs and agents into resolved, skipping duplicates.
fn merge_manifest(resolved: &mut ResolvedManifest, manifest: &ManifestConfig, source: &str) {
    for (name, mcp) in &manifest.mcps {
        if let Some(_existing) = resolved.mcps.get(name) {
            tracing::warn!(
                "MCP '{name}' from {source} conflicts with already-loaded MCP, skipping"
            );
        } else {
            let mut cfg = mcp.clone();
            if cfg.name.is_empty() {
                cfg.name = name.clone();
            }
            resolved.mcps.insert(name.clone(), cfg);
        }
    }

    for (name, agent) in &manifest.agents {
        if let Some(_existing) = resolved.agents.get(name) {
            tracing::warn!(
                "agent '{name}' from {source} conflicts with already-loaded agent, skipping"
            );
        } else {
            resolved.agents.insert(name.clone(), agent.clone());
        }
    }
}

/// Convert a repo URL to a filesystem-safe slug.
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

// ── Agent loading ───────────────────────────────────────────────────

/// Load all agent markdown files from a directory as plain text.
///
/// Returns `(filename_stem, content)` pairs. Non-`.md` files are silently
/// skipped. Entries are sorted by filename for deterministic ordering.
/// Returns an empty vec if the directory does not exist.
pub fn load_agents_dir(path: &Path) -> Result<Vec<(String, String)>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<_> = std::fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut agents = Vec::with_capacity(entries.len());
    for entry in entries {
        let stem = entry
            .path()
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let content = std::fs::read_to_string(entry.path())?;
        agents.push((stem, content));
    }

    Ok(agents)
}

/// Load agents from multiple directories, concatenating results.
/// First directory's agents win on name conflicts (local-first ordering).
pub fn load_agents_dirs(dirs: &[PathBuf]) -> Result<Vec<(String, String)>> {
    let mut seen = std::collections::BTreeSet::new();
    let mut all = Vec::new();
    for dir in dirs {
        for (stem, content) in load_agents_dir(dir)? {
            if seen.insert(stem.clone()) {
                all.push((stem, content));
            }
        }
    }
    Ok(all)
}

// ── Scaffolding ─────────────────────────────────────────────────────

/// Scaffold the full config directory structure on first run.
///
/// Runs migration for old layouts, then creates any missing directories
/// and writes a default `config.toml`.
pub fn scaffold_config_dir(config_dir: &Path) -> Result<()> {
    migrate_layout(config_dir);

    std::fs::create_dir_all(config_dir.join(AGENTS_DIR))
        .context("failed to create agents directory")?;
    std::fs::create_dir_all(config_dir.join(SKILLS_DIR))
        .context("failed to create skills directory")?;
    std::fs::create_dir_all(config_dir.join(PACKAGES_DIR))
        .context("failed to create packages directory")?;

    let config_toml = config_dir.join(CONFIG_FILE);
    if !config_toml.exists() {
        std::fs::write(&config_toml, DEFAULT_CONFIG)
            .with_context(|| format!("failed to write {}", config_toml.display()))?;
    }

    Ok(())
}

// ── Migration ───────────────────────────────────────────────────────

/// Migrate from old config layouts to the new package-centric layout.
///
/// Phase 1: Renames `crab.toml` → `config.toml`, moves `skills/` and
/// `agents/` under `local/`.
///
/// Phase 2: Extracts `[mcps.*]` and `[agents.*]` from `config.toml` into
/// `local/CrabTalk.toml`.
///
/// Each step is a no-op if already migrated. Errors are logged, not fatal.
fn migrate_layout(config_dir: &Path) {
    // Phase 1: rename crab.toml → config.toml
    let old_config = config_dir.join("crab.toml");
    let new_config = config_dir.join(CONFIG_FILE);
    if old_config.exists() && !new_config.exists() {
        if let Err(e) = std::fs::rename(&old_config, &new_config) {
            tracing::warn!("failed to rename crab.toml → config.toml: {e}");
        } else {
            tracing::info!("migrated crab.toml → config.toml");
        }
    }

    let local_dir = config_dir.join(LOCAL_DIR);
    let _ = std::fs::create_dir_all(&local_dir);

    // Phase 1: move skills/ → local/skills/
    let old_skills = config_dir.join("skills");
    let new_skills = config_dir.join(SKILLS_DIR);
    if old_skills.exists() && old_skills.is_dir() && !new_skills.exists() {
        if let Err(e) = std::fs::rename(&old_skills, &new_skills) {
            tracing::warn!("failed to move skills/ → local/skills/: {e}");
        } else {
            tracing::info!("migrated skills/ → local/skills/");
        }
    }

    // Phase 1: move agents/ → local/agents/
    let old_agents = config_dir.join("agents");
    let new_agents = config_dir.join(AGENTS_DIR);
    if old_agents.exists() && old_agents.is_dir() && !new_agents.exists() {
        if let Err(e) = std::fs::rename(&old_agents, &new_agents) {
            tracing::warn!("failed to move agents/ → local/agents/: {e}");
        } else {
            tracing::info!("migrated agents/ → local/agents/");
        }
    }

    // Phase 2: extract [mcps] and [agents] from config.toml → local/CrabTalk.toml
    let config_path = config_dir.join(CONFIG_FILE);
    if config_path.exists() {
        migrate_mcps_agents(&config_path, &local_dir.join("CrabTalk.toml"));
    }
}

/// Extract `[mcps.*]` and `[agents.*]` sections from config.toml into
/// `local/CrabTalk.toml`, removing them from config.toml.
fn migrate_mcps_agents(config_path: &Path, manifest_path: &Path) {
    use toml_edit::DocumentMut;

    let Ok(content) = std::fs::read_to_string(config_path) else {
        return;
    };
    let Ok(mut doc) = content.parse::<DocumentMut>() else {
        return;
    };

    let has_mcps = doc
        .get("mcps")
        .and_then(|v| v.as_table())
        .is_some_and(|t| !t.is_empty());
    let has_agents = doc
        .get("agents")
        .and_then(|v| v.as_table())
        .is_some_and(|t| !t.is_empty());

    if !has_mcps && !has_agents {
        return;
    }

    // Build or load the manifest document.
    let mut manifest_doc = if manifest_path.exists() {
        std::fs::read_to_string(manifest_path)
            .ok()
            .and_then(|s| s.parse::<DocumentMut>().ok())
            .unwrap_or_default()
    } else {
        DocumentMut::default()
    };

    // Only migrate if manifest doesn't already have these sections.
    if has_mcps
        && manifest_doc
            .get("mcps")
            .and_then(|v| v.as_table())
            .is_none_or(|t| t.is_empty())
        && let Some(mcps) = doc.remove("mcps")
    {
        manifest_doc.insert("mcps", mcps);
        tracing::info!("migrated [mcps] from config.toml → local/CrabTalk.toml");
    }

    if has_agents
        && manifest_doc
            .get("agents")
            .and_then(|v| v.as_table())
            .is_none_or(|t| t.is_empty())
        && let Some(agents) = doc.remove("agents")
    {
        manifest_doc.insert("agents", agents);
        tracing::info!("migrated [agents] from config.toml → local/CrabTalk.toml");
    }

    // Write both files back.
    if let Err(e) = std::fs::write(manifest_path, manifest_doc.to_string()) {
        tracing::warn!("failed to write local/CrabTalk.toml: {e}");
        return;
    }
    if let Err(e) = std::fs::write(config_path, doc.to_string()) {
        tracing::warn!("failed to update config.toml after migration: {e}");
    }
}
