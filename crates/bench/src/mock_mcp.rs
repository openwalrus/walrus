//! Mock MCP server — scriptable tool responses over JSON-RPC 2.0.
//!
//! Implements the three MCP methods (`initialize`, `tools/list`, `tools/call`)
//! as a plain axum server. No rmcp dependency needed.

use crate::task::Task;
use axum::{Router, extract::State, http::StatusCode, routing::post};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::net::TcpListener;

/// A recorded tool call for metrics.
#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    pub tool: String,
    pub args: Value,
    pub timestamp: Instant,
}

struct ResponseEntry {
    output: String,
    is_error: bool,
}

/// Mutable server state behind a single mutex.
struct MockInner {
    tool_schemas: Value,
    responses: HashMap<String, Vec<ResponseEntry>>,
    call_counts: HashMap<String, usize>,
    records: Vec<ToolCallRecord>,
}

struct MockState {
    inner: Arc<Mutex<MockInner>>,
}

/// Handle to a running mock MCP server.
pub struct MockMcpHandle {
    addr: SocketAddr,
    inner: Arc<Mutex<MockInner>>,
    shutdown: tokio::sync::oneshot::Sender<()>,
}

impl MockMcpHandle {
    /// Address the server is listening on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Collected tool call records.
    pub fn metrics(&self) -> Vec<ToolCallRecord> {
        self.inner.lock().unwrap().records.clone()
    }

    /// Load a single task's tools and responses, replacing any previous state.
    pub fn load_task(&self, task: &Task) {
        let mut inner = self.inner.lock().unwrap();
        inner.records.clear();
        inner.call_counts.clear();
        inner.responses.clear();
        inner.tool_schemas = json!({
            "tools": task.tools.iter().map(|t| json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.parameters,
            })).collect::<Vec<_>>()
        });
        for resp in &task.responses {
            inner
                .responses
                .entry(resp.tool.to_string())
                .or_default()
                .push(ResponseEntry {
                    output: resp.output.to_string(),
                    is_error: resp.is_error,
                });
        }
    }

    /// Shut down the server.
    pub async fn shutdown(self) {
        let _ = self.shutdown.send(());
    }
}

/// Start the mock MCP server on a specific port (0 = random).
pub async fn start(port: u16, tasks: &[Task]) -> MockMcpHandle {
    // Build initial tool schemas from all tasks (for standalone binary use).
    let mut tool_schemas = Vec::new();
    for task in tasks {
        for tool in &task.tools {
            if !tool_schemas.iter().any(|t: &Value| t["name"] == tool.name) {
                tool_schemas.push(json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": tool.parameters,
                }));
            }
        }
    }

    let inner = Arc::new(Mutex::new(MockInner {
        tool_schemas: json!({ "tools": tool_schemas }),
        responses: HashMap::new(),
        call_counts: HashMap::new(),
        records: Vec::new(),
    }));

    let state = Arc::new(MockState {
        inner: Arc::clone(&inner),
    });

    let app = Router::new()
        .route("/mcp", post(handle_mcp))
        .with_state(state);

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });

    MockMcpHandle {
        addr,
        inner,
        shutdown: shutdown_tx,
    }
}

async fn handle_mcp(
    State(state): State<Arc<MockState>>,
    axum::Json(body): axum::Json<Value>,
) -> (StatusCode, axum::Json<Value>) {
    let id = body.get("id").cloned().unwrap_or(json!(1));
    let method = body["method"].as_str().unwrap_or("");

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-03-26",
            "serverInfo": { "name": "mock-mcp", "version": "0.1.0" },
            "capabilities": { "tools": {} }
        }),

        "tools/list" => state.inner.lock().unwrap().tool_schemas.clone(),

        "tools/call" => {
            let name = body["params"]["name"].as_str().unwrap_or("");
            let args = body["params"]["arguments"].clone();

            let mut inner = state.inner.lock().unwrap();

            // Record the call.
            inner.records.push(ToolCallRecord {
                tool: name.to_string(),
                args: args.clone(),
                timestamp: Instant::now(),
            });

            // Get the next scripted response for this tool.
            let count = inner.call_counts.entry(name.to_string()).or_insert(0);
            let idx = *count;
            *count += 1;

            match inner
                .responses
                .get(name)
                .and_then(|v| v.get(idx).or(v.last()))
            {
                Some(r) => json!({
                    "content": [{ "type": "text", "text": r.output }],
                    "isError": r.is_error
                }),
                None => json!({
                    "content": [{ "type": "text", "text": format!("mock: no response configured for tool '{name}'") }],
                    "isError": true
                }),
            }
        }

        _ => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": format!("unknown method: {method}") }
                })),
            );
        }
    };

    (
        StatusCode::OK,
        axum::Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })),
    )
}
