//! Direct mode — embeds the full gateway stack locally.
//!
//! DirectRunner wraps a `Runtime<GatewayHook>` constructed from gateway.toml,
//! giving CLI users access to all gateway features (SQLite memory, skills,
//! MCP servers, multiple agents).

use crate::config::resolve_config;
use crate::runner::Runner;
use anyhow::Result;
use futures_core::Stream;
use futures_util::StreamExt;
use gateway::GatewayHook;
use llm::Message;
use runtime::Runtime;

/// Runs agents locally using an embedded Runtime with the full gateway stack.
pub struct DirectRunner {
    /// The fully-configured runtime.
    pub runtime: Runtime<GatewayHook>,
}

impl DirectRunner {
    /// Build a DirectRunner from resolved configuration.
    ///
    /// Loads `gateway.toml` (see [`resolve_config`]), constructs the full
    /// runtime (memory backend, provider, skills, MCP, agents).
    pub async fn new() -> Result<Self> {
        let config = resolve_config()?;
        let config_dir = gateway::config::global_config_dir();
        let runtime = gateway::build_runtime(&config, &config_dir).await?;
        Ok(Self { runtime })
    }
}

impl Runner for DirectRunner {
    async fn send(&mut self, agent: &str, content: &str) -> Result<String> {
        let response = self.runtime.send_to(agent, Message::user(content)).await?;
        Ok(response.content().cloned().unwrap_or_default())
    }

    fn stream<'a>(
        &'a mut self,
        agent: &'a str,
        content: &'a str,
    ) -> impl Stream<Item = Result<String>> + Send + 'a {
        let inner = self.runtime.stream_to(agent, Message::user(content));
        inner.map(|result| result.map(|chunk| chunk.content().unwrap_or_default().to_string()))
    }
}
