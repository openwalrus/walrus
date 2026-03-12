//! TCP server — accept loop and per-connection message handler.

use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};
use wcore::protocol::{
    codec,
    message::{client::ClientMessage, server::ServerMessage},
};

/// Accept connections on the given `TcpListener` until shutdown is signalled.
///
/// Each connection is handled in a separate task. For each incoming
/// `ClientMessage`, calls `on_message(msg, reply_tx)` where `reply_tx` is
/// the per-connection sender for streaming `ServerMessage`s back.
pub async fn accept_loop<F>(
    listener: TcpListener,
    on_message: F,
    mut shutdown: oneshot::Receiver<()>,
) where
    F: Fn(ClientMessage, mpsc::UnboundedSender<ServerMessage>) + Clone + Send + 'static,
{
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        let _ = stream.set_nodelay(true);
                        tracing::debug!("tcp connection from {addr}");
                        let cb = on_message.clone();
                        tokio::spawn(async move {
                            let (mut reader, mut writer) = stream.into_split();
                            let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();
                            let send_task = tokio::spawn(async move {
                                while let Some(msg) = rx.recv().await {
                                    if let Err(e) = codec::write_message(&mut writer, &msg).await {
                                        tracing::error!("failed to write message: {e}");
                                        break;
                                    }
                                }
                            });

                            loop {
                                let client_msg: ClientMessage = match codec::read_message(&mut reader).await {
                                    Ok(msg) => msg,
                                    Err(codec::FrameError::ConnectionClosed) => break,
                                    Err(e) => { tracing::debug!("read error: {e}"); break; }
                                };
                                cb(client_msg, tx.clone());
                            }

                            drop(tx);
                            let _ = send_task.await;
                        });
                    }
                    Err(e) => tracing::error!("failed to accept tcp connection: {e}"),
                }
            }
            _ = &mut shutdown => {
                tracing::info!("tcp accept loop shutting down");
                break;
            }
        }
    }
}
