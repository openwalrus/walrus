//! Gateway mode — connect to a running walrus-gateway via WebSocket.

use crate::runner::Runner;
use anyhow::{Result, bail};
use client::{ClientConfig, Connection, WalrusClient};
use compact_str::CompactString;
use futures_core::Stream;
use protocol::{ClientMessage, ServerMessage};

/// Runs agents via a remote walrus-gateway WebSocket connection.
pub struct GatewayRunner {
    connection: Connection,
}

impl GatewayRunner {
    /// Connect to a gateway and optionally authenticate.
    pub async fn connect(url: &str, auth_token: Option<&str>) -> Result<Self> {
        let config = ClientConfig {
            gateway_url: CompactString::from(url),
            auth_token: auth_token.map(CompactString::from),
        };
        let client = WalrusClient::new(config);
        let mut connection = client.connect().await?;

        if let Some(token) = auth_token {
            connection.authenticate(token).await?;
        }

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
