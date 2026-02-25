//! Team composition: register workers as tools in the runtime.
//!
//! Each worker agent is exposed as a tool on the leader. When the leader
//! calls a worker tool, the handler runs a self-contained LLM send loop
//! using captured state (provider, config, memory, agent config, tools,
//! handlers) — no reference back to the Runtime needed.
//!
//! # Example
//!
//! ```rust,ignore
//! use walrus_core::Agent;
//! use walrus_runtime::{Runtime, build_team};
//!
//! let leader = Agent::new("leader").system_prompt("You coordinate.");
//! let analyst = Agent::new("analyst").description("Market analysis");
//!
//! let leader = build_team(leader, vec![analyst], &mut runtime);
//! runtime.add_agent(leader);
//! ```

use agent::{Agent, Memory};
use crate::{Handler, Hook, Provider, MAX_TOOL_CALLS};
use compact_str::CompactString;
use llm::{Config, General, LLM, Message, Tool, ToolChoice};
use std::{collections::BTreeMap, sync::Arc};

/// Build a team: register each worker as a tool and add to the leader.
///
/// Each worker's handler captures everything it needs to independently run
/// an LLM conversation: provider, config, memory, agent config, resolved
/// tool schemas, and resolved tool handlers.
pub fn build_team<H: Hook + 'static>(
    mut leader: Agent,
    workers: Vec<Agent>,
    runtime: &mut crate::Runtime<H>,
) -> Agent {
    let provider = runtime.provider().clone();
    let config = runtime.config().clone();
    let memory = runtime.memory_arc();

    for worker in workers {
        let tool_def = worker_tool(worker.name.clone(), worker.description.to_string());
        let worker_tools = runtime.resolve(&worker.tools);
        let worker_handlers = runtime.resolve_handlers(&worker.tools);

        let p = provider.clone();
        let c = config.clone();
        let m = Arc::clone(&memory);
        let agent = worker.clone();
        let tools = worker_tools;
        let handlers = worker_handlers;

        runtime.register(tool_def, move |args| {
            let p = p.clone();
            let c = c.clone();
            let m = Arc::clone(&m);
            let agent = agent.clone();
            let tools = tools.clone();
            let handlers = handlers.clone();
            async move {
                let input = match extract_input(&args) {
                    Ok(input) => input,
                    Err(e) => return format!("invalid arguments: {e}"),
                };
                worker_send(p, c, m, agent, tools, handlers, input).await
            }
        });

        leader.tools.push(worker.name.clone());
        runtime.add_agent(worker);
    }
    leader
}

/// Run a self-contained LLM send loop for a worker agent.
///
/// Builds the system prompt (base + memory context), sends the input as a
/// user message, and loops through tool calls up to [`MAX_TOOL_CALLS`].
async fn worker_send<M: Memory>(
    provider: Provider,
    config: General,
    memory: Arc<M>,
    agent: Agent,
    tools: Vec<Tool>,
    handlers: BTreeMap<CompactString, Handler>,
    input: String,
) -> String {
    let mut system_prompt = agent.system_prompt.clone();
    let memory_context = memory.compile_relevant(&input).await;
    if !memory_context.is_empty() {
        system_prompt = format!("{system_prompt}\n\n{memory_context}");
    }

    let mut messages = vec![Message::system(&system_prompt), Message::user(&input)];
    let mut tool_choice = ToolChoice::Auto;

    for _ in 0..MAX_TOOL_CALLS {
        let cfg = config
            .clone()
            .with_tools(tools.clone())
            .with_tool_choice(tool_choice.clone());
        let response = match provider.send(&cfg, &messages).await {
            Ok(r) => r,
            Err(e) => return format!("worker error: {e}"),
        };
        let Some(message) = response.message() else {
            return response.content().cloned().unwrap_or_default();
        };

        if message.tool_calls.is_empty() {
            return message.content.clone();
        }

        // Dispatch tool calls using captured handlers.
        let mut tool_results = Vec::with_capacity(message.tool_calls.len());
        for call in &message.tool_calls {
            let output =
                if let Some(handler) = handlers.get(call.function.name.as_str()) {
                    handler(call.function.arguments.clone()).await
                } else {
                    format!("function {} not available", call.function.name)
                };
            tool_results.push(Message::tool(output, call.id.clone()));
        }

        messages.push(message);
        messages.extend(tool_results);
        tool_choice = ToolChoice::None;
    }

    "worker: max tool calls reached".to_string()
}

/// Build a tool definition for a worker agent.
///
/// Uses a standard `{ input: string }` schema so the leader
/// can delegate tasks with a single text field.
pub fn worker_tool(name: impl Into<CompactString>, description: impl Into<String>) -> Tool {
    Tool {
        name: name.into(),
        description: description.into(),
        parameters: default_input_schema(),
        strict: true,
    }
}

/// Extract the `input` field from tool call arguments JSON.
pub fn extract_input(arguments: &str) -> anyhow::Result<String> {
    let parsed: serde_json::Value = serde_json::from_str(arguments)?;
    parsed
        .get("input")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("missing 'input' field in arguments"))
}

/// Default input schema for agent-as-tool calls.
#[derive(schemars::JsonSchema, serde::Deserialize)]
#[allow(dead_code)]
struct DefaultInput {
    /// The task or question to delegate to this agent.
    input: String,
}

fn default_input_schema() -> schemars::Schema {
    schemars::schema_for!(DefaultInput)
}
