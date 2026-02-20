//! Tool runtime: handler registry, dispatch, and compaction.
//!
//! The [`Runtime`] is the single place where `dyn` lives — tool handlers
//! are type-erased closures. Everything else in the agent framework uses
//! static dispatch.

use llm::{Message, Tool, ToolCall};
use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::Arc,
};

/// A type-erased async tool handler.
///
/// Takes the JSON arguments string, returns a result string.
pub type Handler =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = String> + Send>> + Send + Sync>;

/// A compaction function that trims message history.
pub type Compactor = Arc<dyn Fn(Vec<Message>) -> Vec<Message> + Send + Sync>;

/// Tool runtime: registers handlers, resolves tool schemas, dispatches calls.
#[derive(Clone, Default)]
pub struct Runtime {
    tools: BTreeMap<String, (Tool, Handler)>,
    compactors: BTreeMap<String, Compactor>,
}

impl Runtime {
    /// Create an empty runtime.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool with its handler.
    pub fn register<F, Fut>(&mut self, tool: Tool, handler: F)
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = String> + Send + 'static,
    {
        let name = tool.name.clone();
        let handler: Handler = Arc::new(move |args| Box::pin(handler(args)));
        self.tools.insert(name, (tool, handler));
    }

    /// Resolve tool schemas for the given tool names.
    pub fn resolve(&self, names: &[String]) -> Vec<Tool> {
        names
            .iter()
            .filter_map(|name| self.tools.get(name).map(|(tool, _)| tool.clone()))
            .collect()
    }

    /// Dispatch tool calls and collect results as tool messages.
    pub async fn dispatch(&self, calls: &[ToolCall]) -> Vec<Message> {
        let mut results = Vec::with_capacity(calls.len());
        for call in calls {
            let output = if let Some((_, handler)) = self.tools.get(&call.function.name) {
                handler(call.function.arguments.clone()).await
            } else {
                format!("function {} not available", call.function.name)
            };
            results.push(Message::tool(output, &call.id));
        }
        results
    }

    /// Set a compaction function for a specific agent.
    pub fn set_compactor<F>(&mut self, agent: &str, compactor: F)
    where
        F: Fn(Vec<Message>) -> Vec<Message> + Send + Sync + 'static,
    {
        self.compactors
            .insert(agent.to_string(), Arc::new(compactor));
    }

    /// Apply compaction for the given agent. Returns messages unchanged
    /// if no compactor is registered.
    pub fn compact(&self, agent: &str, messages: Vec<Message>) -> Vec<Message> {
        match self.compactors.get(agent) {
            Some(compactor) => compactor(messages),
            None => messages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm::{FunctionCall, ToolCall};

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
        let mut rt = Runtime::new();
        rt.register(echo_tool(), |args| async move { args });
        let tools = rt.resolve(&["echo".into()]);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
    }

    #[test]
    fn resolve_skips_unknown() {
        let rt = Runtime::new();
        let tools = rt.resolve(&["missing".into()]);
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn dispatch_calls_handler() {
        let mut rt = Runtime::new();
        rt.register(echo_tool(), |args| async move {
            format!("got: {args}")
        });

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
        let rt = Runtime::new();
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
        let mut rt = Runtime::new();
        rt.set_compactor("test", |msgs| msgs.into_iter().take(1).collect());

        let msgs = vec![
            Message::user("first"),
            Message::user("second"),
        ];
        let compacted = rt.compact("test", msgs);
        assert_eq!(compacted.len(), 1);
        assert_eq!(compacted[0].content, "first");
    }

    #[test]
    fn no_compactor_passthrough() {
        let rt = Runtime::new();
        let msgs = vec![Message::user("hello")];
        let result = rt.compact("any", msgs.clone());
        assert_eq!(result.len(), 1);
    }
}
