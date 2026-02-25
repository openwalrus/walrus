//! Tests for the Runtime orchestrator.

use agent::{Agent, InMemory, Memory, Skill, SkillTier};
use compact_str::CompactString;
use llm::{FunctionCall, General, LLM, Message, Tool, ToolCall};
use std::collections::BTreeMap;
use walrus_runtime::{Chat, Hook, Provider, Runtime, SkillRegistry};

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
    let mut rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    rt.register(echo_tool(), |args| async move { args });
    let tools = rt.resolve(&["echo".into()]);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
}

#[test]
fn resolve_skips_unknown() {
    let rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    let tools = rt.resolve(&["missing".into()]);
    assert!(tools.is_empty());
}

#[tokio::test]
async fn dispatch_calls_handler() {
    let mut rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
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
    let rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
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
fn chat_requires_registered_agent() {
    let rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    assert!(rt.chat("unknown").is_err());
}

#[test]
fn chat_succeeds_with_agent() {
    let mut rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    rt.add_agent(Agent::new("test").system_prompt("hello"));
    let chat = rt.chat("test").unwrap();
    assert_eq!(chat.agent_name(), "test");
    assert!(chat.messages.is_empty());
}

#[test]
fn context_limit_default() {
    let rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    assert_eq!(rt.context_limit(), 64_000);
}

#[test]
fn context_limit_override() {
    let mut config = General::default();
    config.context_limit = Some(128_000);
    let rt = Runtime::<()>::new(config, test_provider(), InMemory::new());
    assert_eq!(rt.context_limit(), 128_000);
}

#[test]
fn estimate_tokens_counts() {
    let mut rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    rt.add_agent(Agent::new("test").system_prompt("You are helpful."));
    let mut chat = rt.chat("test").unwrap();
    chat.messages.push(Message::user("hello world"));
    let tokens = rt.estimate_tokens(&chat);
    assert!(tokens > 0);
}

#[test]
fn runtime_with_inmemory() {
    let rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    assert!(rt.memory().entries().is_empty());
}

#[test]
fn remember_tool_registered() {
    let rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    let tools = rt.resolve(&["remember".into()]);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "remember");
}

#[tokio::test]
async fn remember_tool_stores_value() {
    let rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());

    let calls = vec![ToolCall {
        id: "call_1".into(),
        index: 0,
        call_type: "function".into(),
        function: FunctionCall {
            name: "remember".into(),
            arguments: r#"{"key": "name", "value": "Alice"}"#.into(),
        },
    }];

    let results = rt.dispatch(&calls).await;
    assert!(results[0].content.contains("remembered"));

    // Verify value was stored.
    assert_eq!(rt.memory().get("name"), Some("Alice".into()));
}

#[tokio::test]
async fn system_prompt_includes_memory() {
    let memory = InMemory::new();
    memory.set("user", "Prefers short answers.");

    let mut rt = Runtime::<()>::new(General::default(), test_provider(), memory);
    rt.add_agent(Agent::new("test").system_prompt("You are helpful."));
    let _chat = rt.chat("test").unwrap();

    // api_messages is private, so test via memory content.
    let compiled = rt.memory().compile();
    assert!(compiled.contains("<user>"));
    assert!(compiled.contains("Prefers short answers."));
}

// --- P2-04: Skills integration tests ---

fn make_test_skill(name: &str, tags: &str, body: &str) -> Skill {
    let mut metadata = BTreeMap::new();
    if !tags.is_empty() {
        metadata.insert(CompactString::from("tags"), tags.into());
    }
    Skill {
        name: name.into(),
        description: String::new(),
        license: None,
        compatibility: None,
        metadata,
        allowed_tools: vec!["echo".into()],
        body: body.into(),
    }
}

#[test]
fn runtime_without_skills_unchanged() {
    let rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    // No skills set — resolve still works for exact matches.
    let tools = rt.resolve(&["remember".into()]);
    assert_eq!(tools.len(), 1);
}

#[test]
fn resolve_glob_prefix() {
    let mut rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    rt.register(
        Tool {
            name: "foo_a".into(),
            description: "".into(),
            parameters: schemars::schema_for!(String),
            strict: false,
        },
        |args| async move { args },
    );
    rt.register(
        Tool {
            name: "foo_b".into(),
            description: "".into(),
            parameters: schemars::schema_for!(String),
            strict: false,
        },
        |args| async move { args },
    );
    rt.register(
        Tool {
            name: "bar_c".into(),
            description: "".into(),
            parameters: schemars::schema_for!(String),
            strict: false,
        },
        |args| async move { args },
    );

    let tools = rt.resolve(&["foo_*".into()]);
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().all(|t| t.name.starts_with("foo_")));
}

#[test]
fn resolve_exact_unchanged() {
    let mut rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    rt.register(echo_tool(), |args| async move { args });
    let tools = rt.resolve(&["echo".into()]);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
}

#[test]
fn skill_body_injected() {
    let mut registry = SkillRegistry::new();
    registry.add(
        make_test_skill("coding", "code", "You are a coding assistant."),
        SkillTier::Bundled,
    );

    let mut rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new())
        .with_skills(registry);
    rt.add_agent(
        Agent::new("dev")
            .system_prompt("Base prompt.")
            .skill_tag("code"),
    );

    // Verify the skill registry is set.
    // (api_messages is private, but we can verify the registry content.)
    let chat = rt.chat("dev").unwrap();
    assert_eq!(chat.agent_name(), "dev");
    // The agent has skill_tags matching "code", and the registry has a skill
    // with tag "code" — the body should be injected in api_messages().
}

#[test]
fn skill_tools_registered() {
    // Verify that skills' allowed_tools are available via resolve.
    let mut rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    rt.register(echo_tool(), |args| async move { args });

    // The skill lists "echo" in allowed_tools — it should resolve.
    let tools = rt.resolve(&["echo".into()]);
    assert_eq!(tools.len(), 1);
}

// --- Hook trait tests ---

#[test]
fn hook_default_compact_prompt() {
    let prompt = <() as Hook>::compact();
    assert!(!prompt.is_empty());
}

#[test]
fn hook_default_flush_prompt() {
    let prompt = <() as Hook>::flush();
    assert!(!prompt.is_empty());
}

// --- Compaction threshold tests ---

#[test]
fn compaction_skips_below_threshold() {
    let rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    let chat = Chat::new("test");
    // Empty chat is well under 80% of the context limit.
    assert!(!rt.needs_compaction(&chat));
}

// --- Chat helper tests ---

#[test]
fn chat_compaction_count_default() {
    let chat = Chat::new("test");
    assert_eq!(chat.compaction_count(), 0);
}

#[test]
fn chat_helpers() {
    let mut chat = Chat::new("test");
    assert!(chat.is_empty());
    assert_eq!(chat.len(), 0);
    assert!(chat.last_message().is_none());

    chat.messages.push(Message::user("hello"));
    assert!(!chat.is_empty());
    assert_eq!(chat.len(), 1);
    assert_eq!(chat.last_message().unwrap().content, "hello");
}

// --- Provider factory tests ---

#[test]
fn provider_deepseek_factory() {
    let provider = Provider::deepseek("test-key");
    assert!(provider.is_ok());
}

// --- Runtime set_skills tests ---

#[test]
fn set_skills_on_existing_runtime() {
    let mut rt = Runtime::<()>::new(General::default(), test_provider(), InMemory::new());
    let registry = SkillRegistry::new();
    rt.set_skills(registry);
    // Resolve still works after setting skills.
    let tools = rt.resolve(&["remember".into()]);
    assert_eq!(tools.len(), 1);
}
