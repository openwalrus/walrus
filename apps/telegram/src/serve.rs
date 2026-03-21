//! Telegram gateway serve logic.

use crate::{
    COMMAND_HINT, DaemonClient, GatewayConfig, GatewayMessage, KnownBots, StreamAccumulator,
    StreamResult, attachment_summary, parse_command,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{collections::HashMap, path::Path, sync::Arc};
use teloxide::prelude::*;
use teloxide::types::ChatAction;
use tokio::sync::mpsc;
use wcore::protocol::message::{
    ClientMessage, ReplyToAsk, ServerMessage, StreamMsg, server_message,
};

/// Run the Telegram gateway service.
pub async fn run(daemon_socket: &str, config: &GatewayConfig) -> anyhow::Result<()> {
    let client = Arc::new(DaemonClient::new(Path::new(daemon_socket)));

    let agents_dir = wcore::paths::CONFIG_DIR.join(wcore::paths::AGENTS_DIR);
    let default_agent = crate::resolve_default_agent(&agents_dir);
    tracing::info!(agent = %default_agent, "telegram gateway starting");

    let known_bots: KnownBots =
        Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new()));

    if let Some(tg) = &config.telegram {
        if tg.token.is_empty() {
            tracing::warn!(platform = "telegram", "token is empty, skipping");
        } else {
            spawn_telegram(
                &tg.token,
                &tg.allowed_users,
                default_agent,
                client,
                known_bots,
            )
            .await;
        }
    } else {
        tracing::warn!(platform = "telegram", "no telegram config provided");
    }

    tokio::signal::ctrl_c().await?;
    tracing::info!("telegram gateway shutting down");
    Ok(())
}

async fn spawn_telegram(
    token: &str,
    allowed_users: &[i64],
    agent: String,
    client: Arc<DaemonClient>,
    known_bots: KnownBots,
) {
    let bot = Bot::new(token);

    match bot.get_me().await {
        Ok(me) => {
            let bot_sender = format!("tg:{}", me.id.0);
            tracing::info!(platform = "telegram", %bot_sender, "registered bot identity");
            known_bots.write().await.insert(bot_sender);
        }
        Err(e) => {
            tracing::warn!(platform = "telegram", "failed to resolve bot identity: {e}");
        }
    }

    // Register slash commands so they appear in the Telegram UI.
    use teloxide::types::BotCommand as TgCommand;
    let commands = vec![TgCommand::new(
        "hub",
        "Manage hub packages (install/uninstall)",
    )];
    if let Err(e) = bot.set_my_commands(commands).await {
        tracing::warn!(
            platform = "telegram",
            "failed to register bot commands: {e}"
        );
    }

    let (tx, rx) = mpsc::unbounded_channel::<GatewayMessage>();

    let poll_bot = bot.clone();
    tokio::spawn(async move {
        crate::poll_loop(poll_bot, tx).await;
    });

    let allowed: std::collections::HashSet<i64> = allowed_users.iter().copied().collect();
    if !allowed.is_empty() {
        tracing::info!(
            platform = "telegram",
            count = allowed.len(),
            "user whitelist active"
        );
    }
    tokio::spawn(telegram_loop(rx, bot, agent, client, known_bots, allowed));
    tracing::info!(platform = "telegram", "channel transport started");
}

/// Per-chat stream state, tracked while a stream is in flight.
struct ChatStream {
    handle: tokio::task::JoinHandle<StreamResult>,
    session_id: Option<u64>,
    reply_tx: mpsc::UnboundedSender<String>,
    waiting_for_reply: Arc<AtomicBool>,
}

impl ChatStream {
    /// Check if the background stream task has finished.
    fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}

/// Reap a finished ChatStream, extracting the session_id on success.
async fn reap_chat(chat: ChatStream) -> Option<u64> {
    match chat.handle.await {
        Ok(StreamResult::Ok { session_id }) => Some(session_id),
        _ => chat.session_id,
    }
}

async fn telegram_loop(
    mut rx: mpsc::UnboundedReceiver<GatewayMessage>,
    bot: Bot,
    agent: String,
    client: Arc<DaemonClient>,
    known_bots: KnownBots,
    allowed_users: std::collections::HashSet<i64>,
) {
    // Per-chat state: active stream tasks and last-known session IDs.
    let mut chats: HashMap<i64, ChatStream> = HashMap::new();
    let mut sessions: HashMap<i64, u64> = HashMap::new();

    while let Some(msg) = rx.recv().await {
        let chat_id = msg.chat_id;
        let content = msg.content.clone();
        let sender = format!("tg:{}", msg.sender_id);

        if known_bots.read().await.contains(&sender) {
            tracing::debug!(%sender, chat_id, "dropping message from known bot");
            continue;
        }

        if !allowed_users.is_empty() && !allowed_users.contains(&msg.sender_id) {
            tracing::debug!(
                sender_id = msg.sender_id,
                chat_id,
                "dropping message from non-allowed user"
            );
            continue;
        }

        // Slash commands are always dispatched immediately.
        if content.starts_with('/') {
            match parse_command(&content) {
                Some(cmd) => {
                    let b = bot.clone();
                    let c = client.clone();
                    tokio::spawn(async move {
                        crate::command::dispatch_command(cmd, c, b, chat_id).await;
                    });
                }
                None => {
                    tracing::warn!(chat_id, content, "unrecognised bot command");
                    if let Err(e) = bot.send_message(ChatId(chat_id), COMMAND_HINT).await {
                        tracing::warn!("failed to send command hint: {e}");
                    }
                }
            }
            continue;
        }

        tracing::info!(agent = %agent, chat_id, "telegram dispatch");

        // Check if there's an active stream for this chat.
        if let Some(chat_stream) = chats.get(&chat_id) {
            if chat_stream.is_finished() {
                // Reap the finished stream and update session.
                let chat_stream = chats.remove(&chat_id).unwrap();
                if let Some(sid) = reap_chat(chat_stream).await {
                    sessions.insert(chat_id, sid);
                }
                // Fall through to spawn a new stream below.
            } else if chat_stream.waiting_for_reply.load(Ordering::Relaxed) {
                // Forward the user's message as a reply to ask_user.
                let _ = chat_stream.reply_tx.send(content);
                continue;
            } else {
                // Stream is active but not waiting — drop the message.
                tracing::warn!(chat_id, "dropping message: stream in progress");
                continue;
            }
        }

        let session = sessions.get(&chat_id).copied();
        let content = match attachment_summary(&msg.attachments) {
            Some(summary) => format!("{content}\n{summary}"),
            None => content,
        };

        // Spawn the stream as a background task.
        let (reply_tx, reply_rx) = mpsc::unbounded_channel();
        let waiting = Arc::new(AtomicBool::new(false));

        let handle = {
            let bot = bot.clone();
            let client = client.clone();
            let agent = agent.clone();
            let waiting = waiting.clone();
            tokio::spawn(async move {
                let result = tg_stream(
                    &bot,
                    &client,
                    &agent,
                    chat_id,
                    msg.message_id,
                    msg.is_group,
                    &content,
                    &sender,
                    session,
                    reply_rx,
                    waiting,
                )
                .await;

                // Handle session retry on error.
                match result {
                    StreamResult::SessionError if session.is_some() => {
                        tracing::warn!(agent = %&agent, chat_id, "session error, retrying");
                        let (_retry_tx, retry_rx) = mpsc::unbounded_channel();
                        let retry_waiting = Arc::new(AtomicBool::new(false));
                        tg_stream(
                            &bot,
                            &client,
                            &agent,
                            chat_id,
                            msg.message_id,
                            msg.is_group,
                            &content,
                            &sender,
                            None,
                            retry_rx,
                            retry_waiting,
                        )
                        .await
                    }
                    other => other,
                }
            })
        };

        chats.insert(
            chat_id,
            ChatStream {
                handle,
                session_id: session,
                reply_tx,
                waiting_for_reply: waiting,
            },
        );
    }

    tracing::info!(platform = "telegram", "channel loop ended");
}

#[allow(clippy::too_many_arguments)]
async fn tg_stream(
    bot: &Bot,
    client: &DaemonClient,
    agent: &str,
    chat_id: i64,
    reply_to_msg_id: i64,
    is_group: bool,
    content: &str,
    sender: &str,
    session: Option<u64>,
    mut reply_rx: mpsc::UnboundedReceiver<String>,
    waiting_for_reply: Arc<AtomicBool>,
) -> StreamResult {
    use std::time::Duration;

    let client_msg = ClientMessage::from(StreamMsg {
        agent: agent.to_string(),
        content: content.to_string(),
        session,
        sender: Some(sender.to_string()),
    });
    let mut server_rx = client.send(client_msg).await;
    let mut acc = StreamAccumulator::new();
    let mut msg_id: Option<teloxide::types::MessageId> = None;
    let mut last_sent_len: usize = 0;
    let mut debounce = tokio::time::interval(Duration::from_millis(1500));
    debounce.reset();

    let typing_bot = bot.clone();
    let typing_handle = tokio::spawn(async move {
        loop {
            if typing_bot
                .send_chat_action(ChatId(chat_id), ChatAction::Typing)
                .await
                .is_err()
            {
                break;
            }
            tokio::time::sleep(Duration::from_secs(4)).await;
        }
    });

    loop {
        tokio::select! {
            server_msg = server_rx.recv() => {
                match server_msg {
                    Some(ServerMessage { msg: Some(server_message::Msg::Stream(event)) }) => {
                        acc.push(&event);

                        // Detect ask_user: render question and signal waiting state.
                        if acc.pending_question().is_some() {
                            // Flush current text first.
                            let rendered = acc.render();
                            if !rendered.is_empty() && rendered.len() != last_sent_len {
                                let reply_to = is_group.then_some(teloxide::types::MessageId(reply_to_msg_id as i32));
                                match msg_id {
                                    None => {
                                        if let Ok(sent) = crate::markdown::send_md(bot, ChatId(chat_id), &rendered, reply_to).await {
                                            msg_id = Some(sent.id);
                                            last_sent_len = rendered.len();
                                        }
                                    }
                                    Some(mid) => {
                                        if crate::markdown::edit_md(bot, ChatId(chat_id), mid, &rendered).await.is_ok() {
                                            last_sent_len = rendered.len();
                                        }
                                    }
                                }
                            }
                            waiting_for_reply.store(true, Ordering::Relaxed);
                        }

                        if acc.is_done() {
                            break;
                        }
                    }
                    Some(ServerMessage { msg: Some(server_message::Msg::Error(err)) }) => {
                        acc.set_error(err.message);
                        break;
                    }
                    Some(_) => {}
                    None => break,
                }
            }
            reply = reply_rx.recv() => {
                if let Some(reply_content) = reply {
                    // Forward the reply to the daemon as ReplyToAsk.
                    if let Some(session_id) = acc.session() {
                        let reply_msg = ClientMessage::from(ReplyToAsk {
                            session: session_id,
                            content: reply_content,
                        });
                        // Fire and forget — the daemon routes it to the pending oneshot.
                        let _ = client.send(reply_msg).await;
                    }
                    acc.take_pending_question();
                    waiting_for_reply.store(false, Ordering::Relaxed);
                }
            }
            _ = debounce.tick() => {
                let rendered = acc.render();
                if rendered.is_empty() || rendered.len() == last_sent_len {
                    continue;
                }
                let reply_to = is_group.then_some(teloxide::types::MessageId(reply_to_msg_id as i32));
                match msg_id {
                    None => {
                        match crate::markdown::send_md(bot, ChatId(chat_id), &rendered, reply_to).await {
                            Ok(sent) => {
                                msg_id = Some(sent.id);
                                last_sent_len = rendered.len();
                            }
                            Err(e) => tracing::warn!(agent, "failed to send placeholder: {e}"),
                        }
                    }
                    Some(mid) => {
                        if let Err(e) = crate::markdown::edit_md(bot, ChatId(chat_id), mid, &rendered).await {
                            tracing::debug!(agent, "edit failed (may be same text): {e}");
                        } else {
                            last_sent_len = rendered.len();
                        }
                    }
                }
            }
        }
    }

    typing_handle.abort();

    if let Some(err) = acc.error() {
        tracing::warn!(agent, chat_id, "stream error: {err}");
        let err_text = format!("Error: {err}");
        if let Err(e) = bot.send_message(ChatId(chat_id), err_text).await {
            tracing::warn!(agent, "failed to send error to chat: {e}");
        }
        return if session.is_some() {
            StreamResult::SessionError
        } else {
            StreamResult::Failed
        };
    }

    let final_text = acc.render();
    if !final_text.is_empty() {
        match msg_id {
            Some(mid) if final_text.len() != last_sent_len => {
                if let Err(e) =
                    crate::markdown::edit_md(bot, ChatId(chat_id), mid, &final_text).await
                {
                    tracing::debug!(agent, "final edit failed: {e}");
                }
            }
            None => {
                let reply_to =
                    is_group.then_some(teloxide::types::MessageId(reply_to_msg_id as i32));
                if let Err(e) =
                    crate::markdown::send_md(bot, ChatId(chat_id), &final_text, reply_to).await
                {
                    tracing::warn!(agent, "failed to send reply: {e}");
                }
            }
            _ => {}
        }
    }

    match acc.session() {
        Some(session_id) => StreamResult::Ok { session_id },
        None => StreamResult::Failed,
    }
}
