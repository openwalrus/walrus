//! Tests for Agent configuration.

use walrus_core::Agent;

#[test]
fn agent_skill_tags_default_empty() {
    let agent = Agent::new("test");
    assert!(agent.skill_tags.is_empty());
}

#[test]
fn agent_skill_tag_builder() {
    let agent = Agent::new("test").skill_tag("analysis").skill_tag("coding");
    assert_eq!(agent.skill_tags.len(), 2);
    assert_eq!(agent.skill_tags[0], "analysis");
    assert_eq!(agent.skill_tags[1], "coding");
}
