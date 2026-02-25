//! Tests for team composition.

use walrus_runtime::{extract_input, worker_tool};

#[test]
fn extract_input_parses_json() {
    let json = r#"{"input": "analyze BTC"}"#;
    assert_eq!(extract_input(json).unwrap(), "analyze BTC");
}

#[test]
fn extract_input_fails_on_missing_field() {
    let json = r#"{"query": "analyze BTC"}"#;
    assert!(extract_input(json).is_err());
}

#[test]
fn extract_input_fails_on_invalid_json() {
    assert!(extract_input("not json").is_err());
}

#[test]
fn worker_tool_builds_tool() {
    let t = worker_tool("analyst", "market analysis");
    assert_eq!(t.name, "analyst");
    assert_eq!(t.description, "market analysis");
    assert!(t.strict);
    let json = serde_json::to_string(&t.parameters).unwrap();
    assert!(json.contains("input"));
}
