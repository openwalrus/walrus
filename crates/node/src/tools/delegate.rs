//! Delegate tool handler factory.

use runtime::{AgentScope, host::Host};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use wcore::{
    ToolDispatch, ToolHandler,
    agent::{AsTool, ToolDescription},
    model::Tool,
};

#[derive(Deserialize, schemars::JsonSchema)]
pub struct Delegate {
    /// List of tasks to run in parallel. Each task has an agent name and a message.
    pub tasks: Vec<DelegateTask>,
    /// If true, return immediately with task IDs instead of waiting for completion.
    /// Results arrive via agent completion events (`agent:{name}:done`).
    #[serde(default)]
    pub background: bool,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct DelegateTask {
    /// Target agent name. Auto-generated if empty and system_prompt is set.
    #[serde(default)]
    pub agent: String,
    /// Message/instruction for the target agent.
    pub message: String,
    /// System prompt for an ephemeral agent. When set, creates a temporary
    /// agent with this prompt instead of targeting a pre-registered agent.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Working directory for this task. Defaults to the parent's CWD.
    #[serde(default)]
    pub cwd: Option<String>,
}

impl ToolDescription for Delegate {
    const DESCRIPTION: &'static str = "Delegate tasks to other agents. Runs all tasks in parallel. Set background=true to return immediately with task IDs — results arrive via agent completion events.";
}

pub fn handler<H: Host + 'static>(
    host: H,
    scopes: Arc<RwLock<BTreeMap<String, AgentScope>>>,
) -> (Tool, ToolHandler) {
    (
        Delegate::as_tool(),
        Arc::new(move |call: ToolDispatch| {
            let host = host.clone();
            let scopes = scopes.clone();
            Box::pin(async move {
                let input: Delegate = serde_json::from_str(&call.args)
                    .map_err(|e| format!("invalid arguments: {e}"))?;
                if input.tasks.is_empty() {
                    return Err("no tasks provided".to_owned());
                }
                {
                    let scopes = scopes.read().expect("scopes lock poisoned");
                    if let Some(scope) = scopes.get(&call.agent)
                        && !scope.members.is_empty()
                    {
                        for task in &input.tasks {
                            if !scope.members.iter().any(|m| m == &task.agent) {
                                return Err(format!(
                                    "agent '{}' is not in your members list",
                                    task.agent
                                ));
                            }
                        }
                    }
                }
                host.dispatch_delegate(&call.args, &call.agent).await
            })
        }),
    )
}
