//! Gateway mode — connect to walrusd via Unix domain socket.

use crate::runner::Runner;
use anyhow::{Result, bail};
use client::{ClientConfig, Connection, WalrusClient};
use compact_str::CompactString;
use futures_core::Stream;
use protocol::{AgentSummary, ClientMessage, ServerMessage};
use std::path::Path;

/// Runs agents via a walrusd Unix domain socket connection.
pub struct GatewayRunner {
    connection: Connection,
}

impl GatewayRunner {
    /// Connect to walrusd.
    pub async fn connect(socket_path: &Path) -> Result<Self> {
        let config = ClientConfig {
            socket_path: socket_path.to_path_buf(),
        };
        let client = WalrusClient::new(config);
        let connection = client.connect().await?;
        Ok(Self { connection })
    }

    /// List all registered agents.
    pub async fn list_agents(&mut self) -> Result<Vec<AgentSummary>> {
        match self.connection.send(ClientMessage::ListAgents).await? {
            ServerMessage::AgentList { agents } => Ok(agents),
            ServerMessage::Error { code, message } => bail!("error ({code}): {message}"),
            other => bail!("unexpected response: {other:?}"),
        }
    }

    /// Get detailed info for a specific agent.
    pub async fn agent_info(&mut self, agent: &str) -> Result<ServerMessage> {
        let msg = ClientMessage::AgentInfo {
            agent: CompactString::from(agent),
        };
        self.connection.send(msg).await
    }

    /// List all memory entries.
    pub async fn list_memory(&mut self) -> Result<Vec<(String, String)>> {
        match self.connection.send(ClientMessage::ListMemory).await? {
            ServerMessage::MemoryList { entries } => Ok(entries),
            ServerMessage::Error { code, message } => bail!("error ({code}): {message}"),
            other => bail!("unexpected response: {other:?}"),
        }
    }

    /// Get a specific memory entry by key.
    pub async fn get_memory(&mut self, key: &str) -> Result<Option<String>> {
        let msg = ClientMessage::GetMemory {
            key: key.to_string(),
        };
        match self.connection.send(msg).await? {
            ServerMessage::MemoryEntry { value, .. } => Ok(value),
            ServerMessage::Error { code, message } => bail!("error ({code}): {message}"),
            other => bail!("unexpected response: {other:?}"),
        }
    }
}

impl Runner for GatewayRunner {
    async fn send(&mut self, agent: &str, content: &str) -> Result<String> {
        let msg = ClientMessage::Send {
            agent: CompactString::from(agent),
            content: content.to_string(),
        };
        match self.connection.send(msg).await? {
            ServerMessage::Response { content, .. } => Ok(content),
            ServerMessage::Error { code, message } => {
                bail!("gateway error ({code}): {message}")
            }
            other => bail!("unexpected response: {other:?}"),
        }
    }

    fn stream<'a>(
        &'a mut self,
        agent: &'a str,
        content: &'a str,
    ) -> impl Stream<Item = Result<String>> + Send + 'a {
        use futures_util::StreamExt;

        let msg = ClientMessage::Stream {
            agent: CompactString::from(agent),
            content: content.to_string(),
        };
        self.connection.stream(msg).filter_map(|result| async {
            match result {
                Ok(ServerMessage::StreamChunk { content }) => Some(Ok(content)),
                Ok(ServerMessage::StreamStart { .. }) => None,
                Ok(ServerMessage::Error { code, message }) => {
                    Some(Err(anyhow::anyhow!("gateway error ({code}): {message}")))
                }
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            }
        })
    }
}
