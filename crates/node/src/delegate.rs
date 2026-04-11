//! Delegate tool handler factory.

use crate::node::SharedRuntime;
use crabllm_core::Provider;
use runtime::{AgentScope, host::Host};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};
use wcore::{
    ToolDispatch, ToolEntry,
    agent::{AsTool, ToolDescription},
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

pub fn handler<P: Provider + 'static, H: Host + 'static>(
    scopes: Arc<RwLock<BTreeMap<String, AgentScope>>>,
    runtime: Arc<OnceLock<SharedRuntime<P, H>>>,
) -> ToolEntry {
    ToolEntry {
        schema: Delegate::as_tool(),
        system_prompt: None,
        before_run: None,
        handler: Arc::new(move |call: ToolDispatch| {
            let scopes = scopes.clone();
            let runtime = runtime.clone();
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
                let shared = runtime
                    .get()
                    .ok_or_else(|| "delegate: runtime not initialized".to_owned())?;
                dispatch_delegate(input, shared).await
            })
        }),
    }
}

async fn dispatch_delegate<P: Provider + 'static, H: Host + 'static>(
    input: Delegate,
    shared: &SharedRuntime<P, H>,
) -> Result<String, String> {
    let mut ephemeral_names = Vec::new();
    let mut tasks = Vec::with_capacity(input.tasks.len());
    for task in input.tasks {
        let agent_name = if let Some(prompt) = task.system_prompt {
            let name = if task.agent.is_empty() {
                ephemeral_agent_name()
            } else {
                task.agent
            };
            let mut config = wcore::AgentConfig::new(&name);
            config.system_prompt = prompt;
            let rt = shared.read().await.clone();
            rt.add_ephemeral(config).await;
            ephemeral_names.push(name.clone());
            name
        } else {
            task.agent
        };

        let sender = delegate_sender();
        let handle = spawn_agent_task(
            shared.clone(),
            agent_name.clone(),
            task.message,
            task.cwd,
            sender.clone(),
        );
        tasks.push((agent_name, sender, handle));
    }

    if input.background {
        let mut json_results = Vec::with_capacity(tasks.len());
        let mut handles = Vec::with_capacity(tasks.len());
        for (agent, sender, handle) in tasks {
            json_results.push(serde_json::json!({ "agent": agent, "task_id": sender }));
            handles.push(handle);
        }
        if !ephemeral_names.is_empty() {
            let shared = shared.clone();
            tokio::spawn(async move {
                for h in handles {
                    let _ = h.await;
                }
                let rt = shared.read().await.clone();
                for name in ephemeral_names {
                    rt.remove_ephemeral(&name).await;
                }
            });
        }
        return serde_json::to_string(&json_results)
            .map_err(|e| format!("serialization error: {e}"));
    }

    let mut results = Vec::with_capacity(tasks.len());
    for (agent_name, _sender, handle) in tasks {
        let (result, error) = match handle.await {
            Ok((r, e)) => (r, e),
            Err(e) => (None, Some(format!("task panicked: {e}"))),
        };
        results.push(serde_json::json!({
            "agent": agent_name,
            "result": result,
            "error": error,
        }));
    }

    if !ephemeral_names.is_empty() {
        let rt = shared.read().await.clone();
        for name in ephemeral_names {
            rt.remove_ephemeral(&name).await;
        }
    }

    serde_json::to_string(&results).map_err(|e| format!("serialization error: {e}"))
}

fn delegate_sender() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("delegate:{id}")
}

fn ephemeral_agent_name() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("_ephemeral:{id}")
}

fn spawn_agent_task<P: Provider + 'static, H: Host + 'static>(
    shared: SharedRuntime<P, H>,
    agent: String,
    message: String,
    cwd: Option<String>,
    delegate_sender: String,
) -> tokio::task::JoinHandle<(Option<String>, Option<String>)> {
    tokio::spawn(async move {
        let rt = shared.read().await.clone();
        let conversation_id = match rt
            .get_or_create_conversation(&agent, &delegate_sender)
            .await
        {
            Ok(id) => id,
            Err(e) => return (None, Some(e.to_string())),
        };
        if let Some(cwd) = cwd {
            rt.hook
                .conversation_cwds
                .lock()
                .await
                .insert(conversation_id, PathBuf::from(cwd));
        }

        let (result_content, error_msg) = match rt
            .send_to(conversation_id, &message, &delegate_sender, None)
            .await
        {
            Ok(response) => (response.final_response, None),
            Err(e) => (None, Some(e.to_string())),
        };

        // Tear down the conversation so the delegate sender identity can
        // be reused. Matches the prior Kill round-trip that went through
        // the protocol layer.
        rt.hook
            .conversation_cwds
            .lock()
            .await
            .remove(&conversation_id);
        rt.close_conversation(conversation_id).await;

        (result_content, error_msg)
    })
}
