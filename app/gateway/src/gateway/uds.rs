//! Unix domain socket server — accept loop and per-connection message handler.

use crate::gateway::Gateway;
use compact_str::CompactString;
use llm::Message;
use protocol::codec::{self, FrameError};
use protocol::{ClientMessage, ServerMessage};
use runtime::Hook;
use std::collections::BTreeMap;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixListener;
use tokio::sync::{mpsc, oneshot};

/// Accept connections on the given `UnixListener` until shutdown is signalled.
pub async fn accept_loop<H: Hook + 'static>(
    listener: UnixListener,
    state: Gateway<H>,
    mut shutdown: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        let state = state.clone();
                        tokio::spawn(async move {
                            handle_connection(stream, state).await;
                        });
                    }
                    Err(e) => {
                        tracing::error!("failed to accept connection: {e}");
                    }
                }
            }
            _ = &mut shutdown => {
                tracing::info!("accept loop shutting down");
                break;
            }
        }
    }
}

/// Handle an established Unix domain socket connection.
async fn handle_connection<H: Hook + 'static>(
    stream: tokio::net::UnixStream,
    state: Gateway<H>,
) {
    let (reader, writer) = stream.into_split();
    let (tx, rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Sender task: forward ServerMessages to the socket.
    let send_task = tokio::spawn(sender_loop(writer, rx));

    // Receiver loop: process incoming ClientMessages.
    receiver_loop(reader, tx, state).await;

    // Clean up — dropping tx already happened in receiver_loop on exit,
    // which causes sender_loop to end.
    let _ = send_task.await;
}

/// Reads messages from the mpsc channel and writes them to the socket.
async fn sender_loop(mut writer: OwnedWriteHalf, mut rx: mpsc::UnboundedReceiver<ServerMessage>) {
    while let Some(msg) = rx.recv().await {
        if let Err(e) = codec::write_message(&mut writer, &msg).await {
            tracing::error!("failed to write message: {e}");
            break;
        }
    }
}

/// Reads client messages from the socket and dispatches them.
async fn receiver_loop<H: Hook + 'static>(
    mut reader: OwnedReadHalf,
    tx: mpsc::UnboundedSender<ServerMessage>,
    state: Gateway<H>,
) {
    let mut session_histories: BTreeMap<CompactString, Vec<Message>> = BTreeMap::new();

    loop {
        let client_msg: ClientMessage = match codec::read_message(&mut reader).await {
            Ok(msg) => msg,
            Err(FrameError::ConnectionClosed) => break,
            Err(e) => {
                tracing::debug!("read error: {e}");
                break;
            }
        };

        match client_msg {
            ClientMessage::Send { agent, content } => {
                let history = session_histories.entry(agent.clone()).or_default();
                match state
                    .runtime
                    .send_stateless(&agent, history, &content)
                    .await
                {
                    Ok(response) => {
                        let _ = tx.send(ServerMessage::Response {
                            agent,
                            content: response,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(ServerMessage::Error {
                            code: 500,
                            message: format!("agent error: {e}"),
                        });
                    }
                }
            }

            ClientMessage::Stream { agent, content } => {
                let _ = tx.send(ServerMessage::StreamStart {
                    agent: agent.clone(),
                });

                let history = session_histories.entry(agent.clone()).or_default();
                match state
                    .runtime
                    .send_stateless(&agent, history, &content)
                    .await
                {
                    Ok(response) => {
                        let _ = tx.send(ServerMessage::StreamChunk { content: response });
                        let _ = tx.send(ServerMessage::StreamEnd { agent });
                    }
                    Err(e) => {
                        let _ = tx.send(ServerMessage::Error {
                            code: 500,
                            message: format!("stream error: {e}"),
                        });
                    }
                }
            }

            ClientMessage::ClearSession { agent } => {
                session_histories.remove(&agent);
                let _ = tx.send(ServerMessage::SessionCleared { agent });
            }

            ClientMessage::Ping => {
                let _ = tx.send(ServerMessage::Pong);
            }
        }
    }
}
