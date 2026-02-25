//! Skill data types following the [agentskills.io](https://agentskills.io/specification) specification.
//!
//! A [`Skill`] is a named, self-contained unit of agent behavior loaded from
//! a `SKILL.md` file with YAML frontmatter. [`SkillTier`] is a runtime
//! concept for resolution priority — not part of the file format.

use compact_str::CompactString;
use std::collections::BTreeMap;

/// Priority tier for skill resolution.
///
/// Variant order defines precedence: Workspace overrides Managed, which
/// overrides Bundled. Assigned by the registry at load time based on
/// source directory — not stored in the skill file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillTier {
    /// Ships with the binary.
    Bundled,
    /// Installed via package manager.
    Managed,
    /// Defined in the project workspace.
    Workspace,
}

/// A named unit of agent behavior (agentskills.io format).
///
/// Pure data struct — parsing and registry logic live in walrus-runtime.
/// Fields mirror the agentskills.io specification. Runtime-only concepts
/// like tier and priority live in the registry, not here.
#[derive(Debug, Clone)]
pub struct Skill {
    /// Skill identifier (lowercase, hyphens, 1-64 chars).
    pub name: CompactString,
    /// Human-readable description (1-1024 chars).
    pub description: String,
    /// SPDX license identifier.
    pub license: Option<CompactString>,
    /// Compatibility constraints (e.g. "walrus>=0.1").
    pub compatibility: Option<CompactString>,
    /// Arbitrary key-value metadata map.
    pub metadata: BTreeMap<CompactString, String>,
    /// Tool names this skill is allowed to use.
    pub allowed_tools: Vec<CompactString>,
    /// Skill body (Markdown instructions).
    pub body: String,
}