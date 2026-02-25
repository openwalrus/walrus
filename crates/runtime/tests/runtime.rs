//! Tests for the Runtime orchestrator.

use llm::{FunctionCall, General, LLM, Message, Tool, ToolCall};
use agent::Agent;
use walrus_runtime::{Provider, Runtime};

fn test_provider() -> Provider {
    Provider::DeepSeek(deepseek::DeepSeek::new(llm::Client::new(), "test-key").unwrap())
}

fn echo_tool() -> Tool {
    Tool {
        name: "echo".into(),
        description: "Echoes the input".into(),
        parameters: schemars::schema_for!(String),
        strict: false,
    }
}

#[test]
fn resolve_returns_registered_tools() {
    let mut rt = Runtime::new(General::default(), test_provider());
    rt.register(echo_tool(), |args| async move { args });
    let tools = rt.resolve(&["echo".into()]);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
}

#[test]
fn resolve_skips_unknown() {
    let rt = Runtime::new(General::default(), test_provider());
    let tools = rt.resolve(&["missing".into()]);
    assert!(tools.is_empty());
}

#[tokio::test]
async fn dispatch_calls_handler() {
    let mut rt = Runtime::new(General::default(), test_provider());
    rt.register(echo_tool(), |args| async move { format!("got: {args}") });

    let calls = vec![ToolCall {
        id: "call_1".into(),
        index: 0,
        call_type: "function".into(),
        function: FunctionCall {
            name: "echo".into(),
            arguments: "hello".into(),
        },
    }];

    let results = rt.dispatch(&calls).await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, "got: hello");
    assert_eq!(results[0].tool_call_id, "call_1");
}

#[tokio::test]
async fn dispatch_unknown_tool() {
    let rt = Runtime::new(General::default(), test_provider());
    let calls = vec![ToolCall {
        id: "call_1".into(),
        index: 0,
        call_type: "function".into(),
        function: FunctionCall {
            name: "missing".into(),
            arguments: "".into(),
        },
    }];

    let results = rt.dispatch(&calls).await;
    assert!(results[0].content.contains("not available"));
}

#[test]
fn compactor_applied() {
    let mut rt = Runtime::new(General::default(), test_provider());
    rt.set_compactor("test", |msgs| msgs.into_iter().take(1).collect());

    let msgs = vec![Message::user("first"), Message::user("second")];
    let compacted = rt.compact("test", msgs);
    assert_eq!(compacted.len(), 1);
    assert_eq!(compacted[0].content, "first");
}

#[test]
fn no_compactor_passthrough() {
    let rt = Runtime::new(General::default(), test_provider());
    let msgs = vec![Message::user("hello")];
    let result = rt.compact("any", msgs.clone());
    assert_eq!(result.len(), 1);
}

#[test]
fn chat_requires_registered_agent() {
    let rt = Runtime::new(General::default(), test_provider());
    assert!(rt.chat("unknown").is_err());
}

#[test]
fn chat_succeeds_with_agent() {
    let mut rt = Runtime::new(General::default(), test_provider());
    rt.add_agent(Agent::new("test").system_prompt("hello"));
    let chat = rt.chat("test").unwrap();
    assert_eq!(chat.agent_name(), "test");
    assert!(chat.messages.is_empty());
}

#[test]
fn context_limit_default() {
    let rt = Runtime::new(General::default(), test_provider());
    assert_eq!(rt.context_limit(), 64_000);
}

#[test]
fn context_limit_override() {
    let mut config = General::default();
    config.context_limit = Some(128_000);
    let rt = Runtime::new(config, test_provider());
    assert_eq!(rt.context_limit(), 128_000);
}

#[test]
fn estimate_tokens_counts() {
    let mut rt = Runtime::new(General::default(), test_provider());
    rt.add_agent(Agent::new("test").system_prompt("You are helpful."));
    let mut chat = rt.chat("test").unwrap();
    chat.messages.push(Message::user("hello world"));
    let tokens = rt.estimate_tokens(&chat);
    assert!(tokens > 0);
}
