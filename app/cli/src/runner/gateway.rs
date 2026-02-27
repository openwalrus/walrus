//! Gateway mode — connect to a running walrus-gateway via Unix domain socket.

use crate::runner::Runner;
use anyhow::{Result, bail};
use client::{ClientConfig, Connection, WalrusClient};
use compact_str::CompactString;
use futures_core::Stream;
use protocol::{ClientMessage, ServerMessage};
use std::path::Path;

/// Runs agents via a remote walrus-gateway Unix domain socket connection.
pub struct GatewayRunner {
    connection: Connection,
}

impl GatewayRunner {
    /// Connect to a gateway.
    pub async fn connect(socket_path: &Path) -> Result<Self> {
        let config = ClientConfig {
            socket_path: socket_path.to_path_buf(),
        };
        let client = WalrusClient::new(config);
        let connection = client.connect().await?;
        Ok(Self { connection })
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
