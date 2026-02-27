//! Unix domain socket connection to the walrus-gateway.

use anyhow::{Result, bail};
use compact_str::CompactString;
use futures_core::Stream;
use protocol::codec;
use protocol::{ClientMessage, ServerMessage};
use std::path::Path;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

/// An established Unix domain socket connection to a walrus-gateway.
///
/// Not Clone — one connection per session. Use [`super::WalrusClient::connect`]
/// to create a connection.
pub struct Connection {
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
}

impl Connection {
    /// Connect to a gateway at the given socket path.
    pub async fn connect(socket_path: &Path) -> Result<Self> {
        let stream = tokio::net::UnixStream::connect(socket_path).await?;
        tracing::debug!("connected to {}", socket_path.display());
        let (reader, writer) = stream.into_split();
        Ok(Self { reader, writer })
    }

    /// Authenticate with the gateway. Returns the session ID.
    pub async fn authenticate(&mut self, token: &str) -> Result<CompactString> {
        let msg = ClientMessage::Authenticate {
            token: token.to_string(),
        };
        match self.send(msg).await? {
            ServerMessage::Authenticated { session_id } => Ok(session_id),
            ServerMessage::Error { code, message } => {
                bail!("authentication failed ({code}): {message}")
            }
            other => bail!("unexpected response to authenticate: {other:?}"),
        }
    }

    /// Send a message and wait for a single response.
    pub async fn send(&mut self, msg: ClientMessage) -> Result<ServerMessage> {
        codec::write_message(&mut self.writer, &msg)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.read_message().await
    }

    /// Send a message and return a stream of server responses.
    ///
    /// The stream yields messages until `StreamEnd` is received or the
    /// connection closes. The `StreamEnd` message itself is not yielded.
    pub fn stream(&mut self, msg: ClientMessage) -> impl Stream<Item = Result<ServerMessage>> + '_ {
        async_stream::try_stream! {
            codec::write_message(&mut self.writer, &msg)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            loop {
                let server_msg = self.read_message().await?;
                match &server_msg {
                    ServerMessage::StreamEnd { .. } => break,
                    ServerMessage::Error { .. } => {
                        yield server_msg;
                        break;
                    }
                    _ => yield server_msg,
                }
            }
        }
    }

    /// Close the connection by dropping both halves.
    pub fn close(self) {
        drop(self);
    }

    /// Read and deserialize a single ServerMessage from the socket.
    async fn read_message(&mut self) -> Result<ServerMessage> {
        codec::read_message(&mut self.reader)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
}
