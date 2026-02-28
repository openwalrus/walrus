//! Tests for the shared OpenAI-compatible Request type.

use walrus_llm::{Config, General, Request, Tool, ToolChoice};

#[test]
fn request_from_general_sets_model() {
    let general = General {
        model: "gpt-4".into(),
        ..General::default()
    };
    let req = Request::from(general);
    assert_eq!(req.model, "gpt-4");
}

#[test]
fn request_from_general_with_tools() {
    let tool = Tool {
        name: "search".into(),
        description: "find docs".into(),
        parameters: schemars::schema_for!(String),
        strict: false,
    };
    let general = General {
        model: "gpt-4".into(),
        tools: Some(vec![tool]),
        ..General::default()
    };
    let req = Request::from(general);
    let tools = req.tools.expect("tools");
    assert_eq!(tools[0]["type"], "function");
    assert_eq!(tools[0]["function"]["name"], "search");
}

#[test]
fn request_with_tool_choice_auto() {
    let req = Request::from(General::default()).with_tool_choice(ToolChoice::Auto);
    assert_eq!(
        req.tool_choice.expect("tool_choice"),
        serde_json::json!("auto")
    );
}

#[test]
fn request_with_tool_choice_none() {
    let req = Request::from(General::default()).with_tool_choice(ToolChoice::None);
    assert_eq!(
        req.tool_choice.expect("tool_choice"),
        serde_json::json!("none")
    );
}

#[test]
fn request_with_tool_choice_required() {
    let req = Request::from(General::default()).with_tool_choice(ToolChoice::Required);
    assert_eq!(
        req.tool_choice.expect("tool_choice"),
        serde_json::json!("required")
    );
}

#[test]
fn request_with_tool_choice_function() {
    let general = General {
        model: "gpt-4".into(),
        tool_choice: Some(ToolChoice::Function("search".into())),
        ..General::default()
    };
    let req = Request::from(general);
    let choice = req.tool_choice.expect("tool_choice");
    assert_eq!(choice["type"], "function");
    assert_eq!(choice["function"]["name"], "search");
}

#[test]
fn request_stream_sets_include_usage() {
    let req = Request::from(General::default()).stream(true);
    assert_eq!(req.stream, Some(true));
    let opts = req.stream_options.expect("stream_options");
    assert_eq!(opts["include_usage"], true);
}

#[test]
fn request_stream_without_usage_omits_stream_options() {
    let req = Request::from(General::default()).stream(false);
    assert_eq!(req.stream, Some(true));
    assert!(req.stream_options.is_none());
}

#[test]
fn request_from_general_thinking_enabled() {
    let general = General {
        model: "deepseek-reasoner".into(),
        think: true,
        ..General::default()
    };
    let req = Request::from(general);
    let thinking = req.thinking.expect("thinking");
    assert_eq!(thinking["type"], "enabled");
}

#[test]
fn request_from_general_thinking_disabled() {
    let general = General {
        model: "gpt-4".into(),
        think: false,
        ..General::default()
    };
    let req = Request::from(general);
    assert!(req.thinking.is_none());
}
