//! RuntimeDispatcher — implements Dispatcher for the Runtime's tool registry.
//!
//! Constructed per-agent with only the tools that agent is configured to use.
//! Dispatches calls through registered handlers and optional MCP bridge.

use crate::{Handler, McpBridge};
use anyhow::Result;
use compact_str::CompactString;
use std::{collections::BTreeMap, future::Future, sync::Arc};
use wcore::Dispatcher;
use wcore::model::Tool;

/// Implements [`Dispatcher`] using the Runtime's tool registry and MCP bridge.
///
/// Created per-agent by resolving the agent's tool names against the full
/// registry. Holds cloned `Arc<Handler>`s and tool schemas. Cheap to construct
/// since handlers are `Arc`-wrapped.
pub struct RuntimeDispatcher {
    tools: Vec<Tool>,
    handlers: BTreeMap<CompactString, Handler>,
    mcp: Option<Arc<McpBridge>>,
}

impl RuntimeDispatcher {
    /// Create a new dispatcher with the given tools, handlers, and optional MCP.
    pub fn new(
        tools: Vec<Tool>,
        handlers: BTreeMap<CompactString, Handler>,
        mcp: Option<Arc<McpBridge>>,
    ) -> Self {
        Self {
            tools,
            handlers,
            mcp,
        }
    }
}

impl Dispatcher for RuntimeDispatcher {
    fn dispatch(&self, calls: &[(&str, &str)]) -> impl Future<Output = Vec<Result<String>>> + Send {
        let calls: Vec<(String, String)> = calls
            .iter()
            .map(|(m, p)| (m.to_string(), p.to_string()))
            .collect();
        let handlers = &self.handlers;
        let mcp = self.mcp.clone();

        async move {
            let mut results = Vec::with_capacity(calls.len());
            for (method, params) in &calls {
                let output = if let Some(handler) = handlers.get(method.as_str()) {
                    Ok(handler(params.clone()).await)
                } else if let Some(ref bridge) = mcp {
                    Ok(bridge.call(method, params).await)
                } else {
                    Ok(format!("function {method} not available"))
                };
                results.push(output);
            }
            results
        }
    }

    fn tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }
}
