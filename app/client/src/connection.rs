//! WebSocket connection to the walrus-gateway.

use crate::ClientConfig;
use anyhow::{Result, bail};
use compact_str::CompactString;
use futures_core::Stream;
use futures_util::{SinkExt, StreamExt};
use protocol::{ClientMessage, ServerMessage};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

/// An established WebSocket connection to a walrus-gateway.
///
/// Not Clone — one connection per session. Use [`WalrusClient::connect`]
/// to create a connection.
pub struct Connection {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Connection {
    /// Connect to a gateway using the given configuration.
    pub async fn connect(config: &ClientConfig) -> Result<Self> {
        let (ws, _response) = connect_async(config.gateway_url.as_str()).await?;
        tracing::debug!("connected to {}", config.gateway_url);
        Ok(Self { ws })
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
        let json = serde_json::to_string(&msg)?;
        self.ws.send(WsMessage::Text(json.into())).await?;
        self.read_message().await
    }

    /// Send a message and return a stream of server responses.
    ///
    /// The stream yields messages until `StreamEnd` is received or the
    /// connection closes. The `StreamEnd` message itself is not yielded.
    pub fn stream(&mut self, msg: ClientMessage) -> impl Stream<Item = Result<ServerMessage>> + '_ {
        async_stream::try_stream! {
            let json = serde_json::to_string(&msg)?;
            self.ws.send(WsMessage::Text(json.into())).await?;

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

    /// Gracefully close the WebSocket connection.
    pub async fn close(mut self) -> Result<()> {
        self.ws.close(None).await?;
        Ok(())
    }

    /// Read and deserialize a single ServerMessage from the WebSocket.
    async fn read_message(&mut self) -> Result<ServerMessage> {
        loop {
            match self.ws.next().await {
                Some(Ok(WsMessage::Text(text))) => {
                    let msg: ServerMessage = serde_json::from_str(&text)?;
                    return Ok(msg);
                }
                Some(Ok(WsMessage::Close(_))) => bail!("connection closed by server"),
                Some(Ok(_)) => continue, // skip binary, ping, pong
                Some(Err(e)) => bail!("websocket error: {e}"),
                None => bail!("connection closed"),
            }
        }
    }
}
