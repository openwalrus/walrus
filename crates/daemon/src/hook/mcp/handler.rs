//! Walrus MCP handler — initial load and read access.

use crate::hook::mcp::{McpBridge, config::McpServerConfig};
use compact_str::CompactString;
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::RwLock;

/// MCP bridge owner.
///
/// MCP bridge owner — `register_tools` registers MCP server tools on the
/// Runtime tool registry. Maintains a sync-accessible cache of server→tools
/// populated at load time so `apply_scope` never needs async bridging.
pub struct McpHandler {
    bridge: RwLock<Arc<McpBridge>>,
    /// Sync cache of server names → tool names, populated at load/reload.
    server_cache: StdRwLock<Vec<(CompactString, Vec<CompactString>)>>,
}

impl McpHandler {
    /// Build a bridge from the given MCP server configs.
    async fn build_bridge(configs: &[McpServerConfig]) -> McpBridge {
        let bridge = McpBridge::new();
        for server_config in configs {
            let mut cmd = tokio::process::Command::new(&server_config.command);
            cmd.args(&server_config.args);
            for (k, v) in &server_config.env {
                cmd.env(k, v);
            }
            tracing::info!(
                server = %server_config.name,
                command = %server_config.command,
                "connecting MCP server"
            );
            match bridge
                .connect_stdio_named(server_config.name.clone(), cmd)
                .await
            {
                Ok(tools) => {
                    tracing::info!(
                        "connected MCP server '{}' — {} tool(s)",
                        server_config.name,
                        tools.len()
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "failed to connect MCP server '{}' (command: {}): {e}",
                        server_config.name,
                        server_config.command
                    );
                }
            }
        }
        bridge
    }

    /// Load MCP servers from the given configs at startup.
    pub async fn load(configs: &[McpServerConfig]) -> Self {
        let bridge = Self::build_bridge(configs).await;
        let servers = bridge.list_servers().await;
        Self {
            bridge: RwLock::new(Arc::new(bridge)),
            server_cache: StdRwLock::new(servers),
        }
    }

    /// List all connected servers with their tool names.
    pub async fn list(&self) -> Vec<(CompactString, Vec<CompactString>)> {
        self.bridge.read().await.list_servers().await
    }

    /// Sync access to the cached server→tools list (populated at load time).
    pub fn cached_list(&self) -> Vec<(CompactString, Vec<CompactString>)> {
        self.server_cache.read().unwrap().clone()
    }

    /// Get a clone of the current bridge Arc.
    pub async fn bridge(&self) -> Arc<McpBridge> {
        Arc::clone(&*self.bridge.read().await)
    }

    /// Try to get a clone of the current bridge Arc without blocking.
    pub fn try_bridge(&self) -> Option<Arc<McpBridge>> {
        self.bridge.try_read().ok().map(|g| Arc::clone(&*g))
    }
}
