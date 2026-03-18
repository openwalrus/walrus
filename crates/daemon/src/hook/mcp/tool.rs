//! Tool dispatch and schema registration for MCP tools.

use crate::hook::DaemonHook;
use schemars::JsonSchema;
use serde::Deserialize;
use wcore::agent::ToolDescription;

#[derive(Deserialize, JsonSchema)]
pub(crate) struct SearchMcp {
    /// Keyword to match tool names and descriptions. Leave empty to list all.
    pub query: String,
}

impl ToolDescription for SearchMcp {
    const DESCRIPTION: &'static str = "Search available MCP tools by keyword.";
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct CallMcpTool {
    /// Tool name
    pub name: String,
    /// JSON-encoded arguments string
    pub args: Option<String>,
}

impl ToolDescription for CallMcpTool {
    const DESCRIPTION: &'static str = "Call an MCP tool by name with JSON-encoded arguments.";
}

impl DaemonHook {
    pub(crate) async fn dispatch_search_mcp(&self, args: &str, agent: &str) -> String {
        let input: SearchMcp = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        let query = input.query.to_lowercase();
        // Get agent's allowed MCP servers for filtering.
        let allowed_mcps = self.scopes.get(agent).map(|s| &s.mcps);
        let bridge = self.mcp.bridge().await;
        // If agent has MCP scope, resolve allowed tool names from server names.
        let allowed_tools: Option<Vec<String>> = if let Some(mcps) = allowed_mcps
            && !mcps.is_empty()
        {
            let servers = bridge.list_servers().await;
            Some(
                servers
                    .into_iter()
                    .filter(|(name, _)| mcps.iter().any(|m| m == name.as_str()))
                    .flat_map(|(_, tools)| tools)
                    .collect(),
            )
        } else {
            None
        };
        let tools = bridge.tools().await;
        let matches: Vec<String> = tools
            .iter()
            .filter(|t| {
                // Filter by agent's MCP scope.
                if let Some(allowed) = &allowed_tools
                    && !allowed.iter().any(|a| a.as_str() == t.name.as_str())
                {
                    return false;
                }
                t.name.to_lowercase().contains(&query)
                    || t.description.to_lowercase().contains(&query)
            })
            .map(|t| format!("{}: {}", t.name, t.description))
            .collect();
        if matches.is_empty() {
            "no tools found".to_owned()
        } else {
            matches.join("\n")
        }
    }

    pub(crate) async fn dispatch_call_mcp_tool(&self, args: &str, agent: &str) -> String {
        let input: CallMcpTool = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };
        // Enforce MCP scope on the tool being called.
        if let Some(scope) = self.scopes.get(agent)
            && !scope.mcps.is_empty()
        {
            let bridge = self.mcp.bridge().await;
            let servers = bridge.list_servers().await;
            let allowed: Vec<String> = servers
                .into_iter()
                .filter(|(name, _)| scope.mcps.iter().any(|m| m == name.as_str()))
                .flat_map(|(_, tools)| tools)
                .collect();
            if !allowed.iter().any(|t| t.as_str() == input.name) {
                return format!("tool not available: {}", input.name);
            }
        }
        let tool_args = input.args.unwrap_or_default();
        let bridge = self.mcp.bridge().await;
        bridge.call(&input.name, &tool_args).await
    }
}
