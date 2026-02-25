//! Tests for Skill types.

use walrus_core::{Skill, SkillTier};

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
