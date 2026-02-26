//! Agent message dispatch -- standalone LLM send loops.
//!
//! Replicates the runtime's send_inner logic using only `&Runtime`
//! (no mutation). The gateway manages its own message histories
//! externally.

use compact_str::CompactString;
use llm::{Config, LLM, Message, Role, Tool, ToolChoice};
use runtime::{Handler, Hook, MAX_TOOL_CALLS, Memory, Runtime};
use std::collections::BTreeMap;

/// Send a message to an agent and return the response content.
///
/// Uses the runtime as a registry for provider, tools, memory, and
/// agent config. Message history is managed externally.
pub async fn agent_send<H: Hook + 'static>(
    runtime: &Runtime<H>,
    messages: &mut Vec<Message>,
    agent_name: &str,
    content: &str,
) -> anyhow::Result<String> {
    let agent = runtime
        .agent(agent_name)
        .ok_or_else(|| anyhow::anyhow!("agent '{agent_name}' not registered"))?;

    // Resolve tools and handlers.
    let resolved = runtime.resolve_tools(&agent.tools);
    let (tools, handlers) = split_resolved(resolved);

    // Build system prompt with memory context.
    let system_prompt = build_system_prompt(runtime, agent_name, content).await;

    // Add user message to history.
    messages.push(Message::user(content));

    // Build API messages: system prompt + history (stripped of reasoning).
    let mut tool_choice = ToolChoice::Auto;
    let base_cfg = runtime.config().clone().with_tools(tools);

    for _ in 0..MAX_TOOL_CALLS {
        let api_messages = build_api_messages(&system_prompt, messages);
        let cfg = base_cfg.clone().with_tool_choice(tool_choice.clone());
        let response = runtime.provider().send(&cfg, &api_messages).await?;

        let Some(message) = response.message() else {
            return Ok(response.content().cloned().unwrap_or_default());
        };

        if message.tool_calls.is_empty() {
            let result = message.content.clone();
            messages.push(message);
            return Ok(result);
        }

        // Dispatch tool calls.
        let tool_results = dispatch_tools(&message, &handlers).await;
        messages.push(message);
        messages.extend(tool_results);
        tool_choice = ToolChoice::None;
    }

    Ok("max tool calls reached".to_string())
}

/// Build the system prompt with memory context.
async fn build_system_prompt<H: Hook + 'static>(
    runtime: &Runtime<H>,
    agent_name: &str,
    user_content: &str,
) -> String {
    let agent = match runtime.agent(agent_name) {
        Some(a) => a,
        None => return String::new(),
    };

    let mut prompt = agent.system_prompt.clone();

    // Append memory context.
    let memory_context = runtime.memory().compile_relevant(user_content).await;
    if !memory_context.is_empty() {
        prompt = format!("{prompt}\n\n{memory_context}");
    }

    prompt
}

/// Build API message list: system prompt + history with reasoning stripped.
fn build_api_messages(system_prompt: &str, history: &[Message]) -> Vec<Message> {
    let needs_system = history.first().map(|m| m.role) != Some(Role::System);
    let extra = if needs_system { 1 } else { 0 };
    let mut messages = Vec::with_capacity(history.len() + extra);

    if needs_system {
        messages.push(Message::system(system_prompt));
    }

    for m in history {
        let mut cloned = m.clone();
        if cloned.tool_calls.is_empty() {
            cloned.reasoning_content = String::new();
        }
        messages.push(cloned);
    }

    messages
}

/// Split resolved tools into schemas and handler map.
fn split_resolved(resolved: Vec<(Tool, Handler)>) -> (Vec<Tool>, BTreeMap<CompactString, Handler>) {
    let mut tools = Vec::with_capacity(resolved.len());
    let mut handlers = BTreeMap::new();
    for (tool, handler) in resolved {
        handlers.insert(tool.name.clone(), handler);
        tools.push(tool);
    }
    (tools, handlers)
}

/// Dispatch tool calls using the handler map.
async fn dispatch_tools(
    message: &Message,
    handlers: &BTreeMap<CompactString, Handler>,
) -> Vec<Message> {
    let mut results = Vec::with_capacity(message.tool_calls.len());
    for call in &message.tool_calls {
        let output = if let Some(handler) = handlers.get(call.function.name.as_str()) {
            handler(call.function.arguments.clone()).await
        } else {
            format!("function {} not available", call.function.name)
        };
        results.push(Message::tool(output, call.id.clone()));
    }
    results
}
