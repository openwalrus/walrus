//! MCP tool handler factory.

use runtime::{AgentScope, host::Host};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use wcore::{ToolDispatch, ToolHandler};

/// Build a handler that dispatches MCP tool calls through the host.
pub fn handler<H: Host + 'static>(
    host: H,
    scopes: Arc<RwLock<BTreeMap<String, AgentScope>>>,
) -> ToolHandler {
    Arc::new(move |call: ToolDispatch| {
        let host = host.clone();
        let scopes = scopes.clone();
        Box::pin(async move {
            let allowed_mcps: Vec<String> = scopes
                .read()
                .expect("scopes lock poisoned")
                .get(&call.agent)
                .filter(|s| !s.mcps.is_empty())
                .map(|s| s.mcps.clone())
                .unwrap_or_default();
            host.dispatch_mcp(&call.args, &allowed_mcps).await
        })
    })
}
