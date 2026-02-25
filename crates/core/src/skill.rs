//! Skill data types.
//!
//! A [`Skill`] is a named, self-contained unit of agent behavior.
//! Skills are loaded and managed by the SkillRegistry in walrus-runtime.

use compact_str::CompactString;

/// Priority tier for skill resolution.
///
/// Variant order defines precedence: Workspace overrides Managed, which
/// overrides Bundled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillTier {
    /// Ships with the binary.
    Bundled,
    /// Installed via package manager.
    Managed,
    /// Defined in the project workspace.
    Workspace,
}

/// A named unit of agent behavior.
///
/// Pure data struct — parsing and registry logic live in walrus-runtime.
#[derive(Debug, Clone)]
pub struct Skill {
    /// Skill identifier.
    pub name: CompactString,
    /// Human-readable description.
    pub description: String,
    /// Semantic version string.
    pub version: String,
    /// Priority tier (Bundled < Managed < Workspace).
    pub tier: SkillTier,
    /// Tags for matching skills to agents.
    pub tags: Vec<CompactString>,
    /// Trigger patterns that activate this skill.
    pub triggers: Vec<CompactString>,
    /// Tool names this skill provides or requires.
    pub tools: Vec<CompactString>,
    /// Priority within the same tier (0-255, higher wins).
    pub priority: u8,
    /// Skill body (prompt template or executable content).
    pub body: String,
}

#[cfg(test)]
mod tests {
    use crate::skill::{Skill, SkillTier};

    #[test]
    fn skill_tier_ordering() {
        assert!(SkillTier::Bundled < SkillTier::Managed);
        assert!(SkillTier::Managed < SkillTier::Workspace);
        assert!(SkillTier::Bundled < SkillTier::Workspace);
    }

    #[test]
    fn skill_has_tier_field() {
        let skill = Skill {
            name: "test".into(),
            description: String::new(),
            version: "0.1.0".into(),
            tier: SkillTier::Workspace,
            tags: vec![],
            triggers: vec![],
            tools: vec![],
            priority: 10,
            body: String::new(),
        };
        assert_eq!(skill.tier, SkillTier::Workspace);
        assert_eq!(skill.priority, 10);
    }
}
