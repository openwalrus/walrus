//! Markdown-based configuration loading for agents and cron jobs.
//!
//! Agents and cron jobs are defined as markdown files with YAML frontmatter,
//! following the same pattern as skills. The frontmatter contains structured
//! fields and the markdown body becomes the system prompt (agents) or
//! message template (cron).

use wcore::Agent;
use compact_str::CompactString;
use serde::Deserialize;
use std::path::Path;

/// YAML frontmatter for agent markdown files.
#[derive(Deserialize)]
struct AgentFrontmatter {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    skill_tags: Vec<String>,
}

/// A cron job entry parsed from a markdown file.
#[derive(Debug, Clone)]
pub struct CronEntry {
    /// Cron job name.
    pub name: CompactString,
    /// Cron schedule expression (e.g. "0 0 9 * * *").
    pub schedule: String,
    /// Name of the agent to invoke.
    pub agent: CompactString,
    /// Message template (from the markdown body).
    pub message: String,
}

/// YAML frontmatter for cron markdown files.
#[derive(Deserialize)]
struct CronFrontmatter {
    name: String,
    schedule: String,
    agent: String,
}

/// Parse an agent markdown file (YAML frontmatter + body) into an [`Agent`].
///
/// The frontmatter provides name, description, tools, and skill_tags.
/// The markdown body (trimmed) becomes the agent's system prompt.
pub fn parse_agent_md(content: &str) -> anyhow::Result<Agent> {
    let (frontmatter, body) = crate::skills::split_yaml_frontmatter(content)?;
    let fm: AgentFrontmatter = serde_yaml::from_str(frontmatter)?;

    let mut agent = Agent::new(fm.name)
        .description(fm.description)
        .system_prompt(body.trim().to_owned());
    for tool in fm.tools {
        agent = agent.tool(tool);
    }
    for tag in fm.skill_tags {
        agent = agent.skill_tag(tag);
    }

    Ok(agent)
}

/// Load all agent markdown files from a directory.
///
/// Each `.md` file is parsed with [`parse_agent_md`]. Non-`.md` files are
/// silently skipped. Entries are sorted by filename for deterministic ordering.
/// Returns an empty vec if the directory does not exist.
pub fn load_agents_dir(path: &Path) -> anyhow::Result<Vec<Agent>> {
    if !path.exists() {
        tracing::warn!("agent directory does not exist: {}", path.display());
        return Ok(Vec::new());
    }

    let mut entries: Vec<_> = std::fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut agents = Vec::with_capacity(entries.len());
    for entry in entries {
        let content = std::fs::read_to_string(entry.path())?;
        agents.push(parse_agent_md(&content)?);
    }

    Ok(agents)
}

/// Parse a cron markdown file (YAML frontmatter + body) into a [`CronEntry`].
///
/// The frontmatter provides name, schedule, and agent. The markdown body
/// (trimmed) becomes the cron entry's message template.
pub fn parse_cron_md(content: &str) -> anyhow::Result<CronEntry> {
    let (frontmatter, body) = crate::skills::split_yaml_frontmatter(content)?;
    let fm: CronFrontmatter = serde_yaml::from_str(frontmatter)?;

    Ok(CronEntry {
        name: CompactString::from(fm.name),
        schedule: fm.schedule,
        agent: CompactString::from(fm.agent),
        message: body.trim().to_owned(),
    })
}

/// Load all cron markdown files from a directory.
///
/// Each `.md` file is parsed with [`parse_cron_md`]. Non-`.md` files are
/// silently skipped. Entries are sorted by filename for deterministic ordering.
/// Returns an empty vec if the directory does not exist.
pub fn load_cron_dir(path: &Path) -> anyhow::Result<Vec<CronEntry>> {
    if !path.exists() {
        tracing::warn!("cron directory does not exist: {}", path.display());
        return Ok(Vec::new());
    }

    let mut entries: Vec<_> = std::fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut crons = Vec::with_capacity(entries.len());
    for entry in entries {
        let content = std::fs::read_to_string(entry.path())?;
        crons.push(parse_cron_md(&content)?);
    }

    Ok(crons)
}
