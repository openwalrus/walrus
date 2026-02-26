//! Tests for CLI management command logic.

use agent::{Agent, InMemory};
use llm::{General, NoopProvider};
use runtime::Runtime;

#[test]
fn agent_list_output() {
    let mut rt = Runtime::<()>::new(General::default(), NoopProvider, InMemory::new());
    rt.add_agent(Agent::new("alice").description("Alice agent"));
    rt.add_agent(Agent::new("bob").description("Bob agent"));

    let agents: Vec<_> = rt.agents().collect();
    assert_eq!(agents.len(), 2);

    let names: Vec<_> = agents.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"alice"));
    assert!(names.contains(&"bob"));
}
