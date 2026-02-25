//! Tests for Skill types.

use walrus_core::{Skill, SkillTier};

#[test]
fn skill_tier_ordering() {
    assert!(SkillTier::Bundled < SkillTier::Managed);
    assert!(SkillTier::Managed < SkillTier::Workspace);
    assert!(SkillTier::Bundled < SkillTier::Workspace);
}

#[test]
fn skill_fields_match_spec() {
    let mut metadata = std::collections::BTreeMap::new();
    metadata.insert("tags".into(), "coding,rust".into());
    metadata.insert("triggers".into(), "help me code".into());

    let skill = Skill {
        name: "code-assistant".into(),
        description: "Helps write Rust code.".into(),
        license: Some("MIT".into()),
        compatibility: Some("walrus>=0.1".into()),
        metadata,
        allowed_tools: vec!["bash".into(), "read_file".into()],
        body: "You are a coding assistant.".into(),
    };

    assert_eq!(skill.name.as_str(), "code-assistant");
    assert_eq!(skill.license.as_deref(), Some("MIT"));
    assert_eq!(skill.compatibility.as_deref(), Some("walrus>=0.1"));
    assert_eq!(skill.metadata.len(), 2);
    assert_eq!(skill.metadata.get("tags").unwrap(), "coding,rust");
    assert_eq!(skill.allowed_tools.len(), 2);
    assert!(!skill.body.is_empty());
}
