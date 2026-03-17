//! Tool dispatch and schema registration for task tools.

use crate::hook::{DaemonHook, system::task::TaskStatus};
use serde::Deserialize;
use wcore::{
    agent::{AsTool, ToolDescription},
    model::Tool,
};

impl DaemonHook {
    pub(crate) async fn dispatch_spawn_task(
        &self,
        args: &str,
        agent: &str,
        parent_task_id: Option<u64>,
    ) -> String {
        let input: SpawnTask = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        // Enforce members scope — reject if caller has a members list and target is not in it.
        if let Some(scope) = self.scopes.get(agent)
            && !scope.members.is_empty()
            && !scope.members.iter().any(|m| m == &input.agent)
        {
            return format!("agent '{}' is not in your members list", input.agent);
        }
        let registry = self.tasks.clone();
        let (task_id, status) = registry.lock().await.submit(
            input.agent.into(),
            input.message,
            agent.into(),
            parent_task_id,
            registry.clone(),
        );
        serde_json::json!({ "task_id": task_id, "status": status.to_string() }).to_string()
    }

    pub(crate) async fn dispatch_check_tasks(&self, args: &str) -> String {
        let input: CheckTasks = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        let status_filter = input.status.as_deref().and_then(parse_task_status);
        let registry = self.tasks.lock().await;
        let tasks = registry.list(
            input.agent.as_deref(),
            status_filter,
            input.parent_id.map(Some),
        );
        let entries: Vec<serde_json::Value> = tasks
            .iter()
            .map(|t| {
                serde_json::json!({
                    "task_id": t.id,
                    "agent": t.agent.as_str(),
                    "status": t.status.to_string(),
                    "description": t.description,
                    "parent_id": t.parent_id,
                    "result": t.result,
                    "error": t.error,
                    "created_by": t.created_by.as_str(),
                    "alive_secs": t.created_at.elapsed().as_secs(),
                    "prompt_tokens": t.prompt_tokens,
                    "completion_tokens": t.completion_tokens,
                })
            })
            .collect();
        serde_json::to_string(&entries).unwrap_or_else(|e| format!("serialization error: {e}"))
    }

    pub(crate) async fn dispatch_ask_user(&self, args: &str, task_id: Option<u64>) -> String {
        let input: AskUser = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        let Some(tid) = task_id else {
            return "ask_user can only be called from within a task context".to_owned();
        };
        let rx = {
            let mut registry = self.tasks.lock().await;
            match registry.block(tid, input.question) {
                Some(rx) => rx,
                None => return format!("task {tid} not found"),
            }
        };
        match rx.await {
            Ok(response) => response,
            Err(_) => "user did not respond (channel closed)".to_owned(),
        }
    }

    pub(crate) async fn dispatch_await_tasks(&self, args: &str, task_id: Option<u64>) -> String {
        let input: AwaitTasks = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        if input.task_ids.is_empty() {
            return "no task IDs provided".to_owned();
        }
        // Subscribe to status changes for each requested task.
        let mut receivers = Vec::new();
        {
            let registry = self.tasks.lock().await;
            for &tid in &input.task_ids {
                match registry.subscribe_status(tid) {
                    Some(rx) => receivers.push((tid, rx)),
                    None => return format!("task {tid} not found"),
                }
            }
        }
        // If running in a task context, mark ourselves as blocked.
        if let Some(tid) = task_id {
            let mut registry = self.tasks.lock().await;
            registry.set_status(tid, TaskStatus::Blocked);
        }
        // Wait for all tasks to reach Finished or Failed.
        for (_, rx) in &mut receivers {
            let mut rx = rx.clone();
            loop {
                let status = *rx.borrow_and_update();
                if status == TaskStatus::Finished || status == TaskStatus::Failed {
                    break;
                }
                if rx.changed().await.is_err() {
                    break;
                }
            }
        }
        // Unblock ourselves.
        if let Some(tid) = task_id {
            let mut registry = self.tasks.lock().await;
            registry.set_status(tid, TaskStatus::InProgress);
        }
        // Collect results.
        let registry = self.tasks.lock().await;
        let results: Vec<serde_json::Value> = input
            .task_ids
            .iter()
            .map(|&tid| {
                if let Some(t) = registry.get(tid) {
                    serde_json::json!({
                        "task_id": tid,
                        "status": t.status.to_string(),
                        "result": t.result,
                        "error": t.error,
                    })
                } else {
                    serde_json::json!({ "task_id": tid, "status": "not_found" })
                }
            })
            .collect();
        serde_json::to_string(&results).unwrap_or_else(|e| format!("serialization error: {e}"))
    }
}

/// Parse a status string into a `TaskStatus`.
fn parse_task_status(s: &str) -> Option<TaskStatus> {
    match s {
        "queued" => Some(TaskStatus::Queued),
        "in_progress" => Some(TaskStatus::InProgress),
        "blocked" => Some(TaskStatus::Blocked),
        "finished" => Some(TaskStatus::Finished),
        "failed" => Some(TaskStatus::Failed),
        _ => None,
    }
}

/// Task tools.
pub(crate) fn tools() -> Vec<Tool> {
    vec![
        SpawnTask::as_tool(),
        CheckTasks::as_tool(),
        AskUser::as_tool(),
        AwaitTasks::as_tool(),
    ]
}

#[derive(Deserialize, schemars::JsonSchema)]
pub(crate) struct SpawnTask {
    /// Target agent name to delegate the task to.
    pub agent: String,
    /// Message/instruction for the target agent.
    pub message: String,
}

impl ToolDescription for SpawnTask {
    const DESCRIPTION: &'static str = "Delegate an async task to another agent. Returns task_id and status (in_progress or queued). Use check_tasks to monitor progress.";
}

#[derive(Deserialize, schemars::JsonSchema)]
pub(crate) struct CheckTasks {
    /// Filter by agent name.
    #[serde(default)]
    pub agent: Option<String>,
    /// Filter by status (queued, in_progress, blocked, finished, failed).
    #[serde(default)]
    pub status: Option<String>,
    /// Filter by parent task ID.
    #[serde(default)]
    pub parent_id: Option<u64>,
}

impl ToolDescription for CheckTasks {
    const DESCRIPTION: &'static str = "Query the task registry. Filterable by agent, status, parent_id. Returns up to 16 most recent tasks.";
}

#[derive(Deserialize, schemars::JsonSchema)]
pub(crate) struct AskUser {
    /// Question to ask the user.
    pub question: String,
}

impl ToolDescription for AskUser {
    const DESCRIPTION: &'static str = "Ask the user a question. Blocks the current task until the user responds. Only works within a task context.";
}

#[derive(Deserialize, schemars::JsonSchema)]
pub(crate) struct AwaitTasks {
    /// Task IDs to wait for.
    pub task_ids: Vec<u64>,
}

impl ToolDescription for AwaitTasks {
    const DESCRIPTION: &'static str =
        "Block until the specified tasks finish. Returns collected results for each task.";
}
