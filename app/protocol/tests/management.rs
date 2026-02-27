//! Management protocol message tests.

use walrus_protocol::{AgentSummary, ClientMessage, ServerMessage};

#[test]
fn client_list_agents_roundtrip() {
    let msg = ClientMessage::ListAgents;
    let json = serde_json::to_string(&msg).unwrap();
    let back: ClientMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, ClientMessage::ListAgents));
}

#[test]
fn client_agent_info_roundtrip() {
    let msg = ClientMessage::AgentInfo {
        agent: "helper".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: ClientMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, ClientMessage::AgentInfo { agent } if agent == "helper"));
}

#[test]
fn client_list_memory_roundtrip() {
    let msg = ClientMessage::ListMemory;
    let json = serde_json::to_string(&msg).unwrap();
    let back: ClientMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, ClientMessage::ListMemory));
}

#[test]
fn client_get_memory_roundtrip() {
    let msg = ClientMessage::GetMemory { key: "fact".into() };
    let json = serde_json::to_string(&msg).unwrap();
    let back: ClientMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, ClientMessage::GetMemory { key } if key == "fact"));
}

#[test]
fn server_agent_list_roundtrip() {
    let msg = ServerMessage::AgentList {
        agents: vec![
            AgentSummary {
                name: "assistant".into(),
                description: "A helpful assistant".into(),
            },
            AgentSummary {
                name: "coder".into(),
                description: "A coding agent".into(),
            },
        ],
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: ServerMessage = serde_json::from_str(&json).unwrap();
    match back {
        ServerMessage::AgentList { agents } => {
            assert_eq!(agents.len(), 2);
            assert_eq!(agents[0].name.as_str(), "assistant");
            assert_eq!(agents[1].name.as_str(), "coder");
        }
        _ => panic!("expected AgentList"),
    }
}

#[test]
fn server_agent_detail_roundtrip() {
    let msg = ServerMessage::AgentDetail {
        name: "helper".into(),
        description: "A helper".into(),
        tools: vec!["remember".into()],
        skill_tags: vec!["search".into()],
        system_prompt: "You are helpful.".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: ServerMessage = serde_json::from_str(&json).unwrap();
    match back {
        ServerMessage::AgentDetail {
            name,
            description,
            tools,
            skill_tags,
            system_prompt,
        } => {
            assert_eq!(name.as_str(), "helper");
            assert_eq!(description.as_str(), "A helper");
            assert_eq!(tools.len(), 1);
            assert_eq!(skill_tags.len(), 1);
            assert_eq!(system_prompt, "You are helpful.");
        }
        _ => panic!("expected AgentDetail"),
    }
}

#[test]
fn server_memory_list_roundtrip() {
    let msg = ServerMessage::MemoryList {
        entries: vec![
            ("key1".into(), "val1".into()),
            ("key2".into(), "val2".into()),
        ],
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: ServerMessage = serde_json::from_str(&json).unwrap();
    match back {
        ServerMessage::MemoryList { entries } => {
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].0, "key1");
        }
        _ => panic!("expected MemoryList"),
    }
}

#[test]
fn server_memory_entry_found_roundtrip() {
    let msg = ServerMessage::MemoryEntry {
        key: "fact".into(),
        value: Some("the sky is blue".into()),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: ServerMessage = serde_json::from_str(&json).unwrap();
    match back {
        ServerMessage::MemoryEntry { key, value } => {
            assert_eq!(key, "fact");
            assert_eq!(value.as_deref(), Some("the sky is blue"));
        }
        _ => panic!("expected MemoryEntry"),
    }
}

#[test]
fn server_memory_entry_not_found_roundtrip() {
    let msg = ServerMessage::MemoryEntry {
        key: "missing".into(),
        value: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: ServerMessage = serde_json::from_str(&json).unwrap();
    match back {
        ServerMessage::MemoryEntry { key, value } => {
            assert_eq!(key, "missing");
            assert!(value.is_none());
        }
        _ => panic!("expected MemoryEntry"),
    }
}
