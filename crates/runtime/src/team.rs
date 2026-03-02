//! Team composition: register workers as tools in the runtime.
//!
//! Each worker agent is exposed as a tool on the leader. When the leader
//! calls a worker tool, the handler creates an ephemeral Agent instance
//! and runs it with a captured RuntimeDispatcher.
//!
//! # Example
//!
//! ```rust,ignore
//! use walrus_core::AgentConfig;
//! use walrus_runtime::{Runtime, build_team};
//!
//! let leader = AgentConfig::new("leader").system_prompt("You coordinate.");
//! let analyst = AgentConfig::new("analyst").description("Market analysis");
//!
//! let leader = build_team(leader, vec![analyst], &mut runtime);
//! runtime.add_agent(leader);
//! ```

use crate::{Hook, RuntimeDispatcher, SkillRegistry};
use compact_str::CompactString;
use std::sync::Arc;
use wcore::AgentConfig;
use wcore::model::{Message, Model, Tool};

/// Build a team: register each worker as a tool and add to the leader.
///
/// Each worker's handler captures everything it needs to independently run
/// a conversation: provider, agent config, and a RuntimeDispatcher.
/// Workers create ephemeral Agent instances per invocation (no cross-call state).
pub fn build_team<H: Hook + 'static>(
    mut leader: AgentConfig,
    workers: Vec<AgentConfig>,
    runtime: &mut crate::Runtime<H>,
) -> AgentConfig {
    let provider = runtime.provider().clone();
    let skills = runtime.skills().map(Arc::clone);

    for worker in workers {
        let tool_def = worker_tool(worker.name.clone(), worker.description.to_string());
        let dispatcher = runtime.build_dispatcher(&worker);

        let ctx = Arc::new(WorkerCtx {
            provider: provider.clone(),
            skills: skills.clone(),
            agent: worker.clone(),
            dispatcher,
        });

        runtime.register(tool_def, move |args| {
            let ctx = Arc::clone(&ctx);
            async move {
                let input = match extract_input(&args) {
                    Ok(input) => input,
                    Err(e) => return format!("invalid arguments: {e}"),
                };
                worker_send(&ctx, input).await
            }
        });

        leader.tools.push(worker.name.clone());
        runtime.add_agent(worker);
    }
    leader
}

/// Shared immutable state for a worker handler.
struct WorkerCtx<P: Model> {
    provider: P,
    skills: Option<Arc<SkillRegistry>>,
    agent: AgentConfig,
    dispatcher: RuntimeDispatcher,
}

/// Run a worker agent using Agent.run().
///
/// Creates an ephemeral Agent with a fresh event channel, enriches the
/// system prompt with skills, pushes the user input, and runs to completion.
async fn worker_send<P: Model>(ctx: &WorkerCtx<P>, input: String) -> String {
    let mut config = ctx.agent.clone();

    // Inject skill bodies matching the agent's skill tags.
    if let Some(registry) = &ctx.skills {
        for skill in registry.find_by_tags(&config.skill_tags) {
            if !skill.body.is_empty() {
                config.system_prompt.push_str("\n\n");
                config.system_prompt.push_str(&skill.body);
            }
        }
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let mut agent = wcore::AgentBuilder::new(tx).config(config).build();
    agent.push_message(Message::user(&input));

    // Drain events (discard for workers).
    tokio::spawn(async move { while rx.recv().await.is_some() {} });

    let response = agent.run(&ctx.provider, &ctx.dispatcher).await;
    response.final_response.unwrap_or_default()
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
