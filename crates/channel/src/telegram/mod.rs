//! Telegram Bot API adapter.
//!
//! Uses teloxide long-polling for receiving messages and `Bot::send_message`
//! for sending replies.

pub(crate) mod command;

use crate::message::{Attachment, AttachmentKind, ChannelMessage};
use futures_util::StreamExt;
use teloxide::prelude::*;
use teloxide::types::UpdateKind;
use teloxide::update_listeners::{AsUpdateStream, polling_default};
use tokio::sync::mpsc;

/// Long-poll loop: receives Telegram updates and forwards them as [`ChannelMessage`]s.
pub(crate) async fn poll_loop(bot: Bot, tx: mpsc::UnboundedSender<ChannelMessage>) {
    let mut listener = polling_default(bot).await;
    let stream = listener.as_stream();
    futures_util::pin_mut!(stream);

    while let Some(result) = stream.next().await {
        match result {
            Ok(update) => {
                if let Some(msg) = convert_update(update)
                    && tx.send(msg).is_err()
                {
                    tracing::info!("channel handle dropped, stopping poll loop");
                    return;
                }
            }
            Err(e) => {
                tracing::error!("telegram update error: {e}");
            }
        }
    }
}

/// Convert a teloxide `Update` to a `ChannelMessage`.
fn convert_update(update: Update) -> Option<ChannelMessage> {
    let UpdateKind::Message(msg) = update.kind else {
        return None;
    };

    let chat_id = msg.chat.id.0;
    let sender_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let content = msg.text().unwrap_or("").to_owned();

    let mut attachments = Vec::new();
    if let Some(photos) = msg.photo()
        && let Some(largest) = photos.last()
    {
        attachments.push(Attachment {
            kind: AttachmentKind::Image,
            url: largest.file.id.0.clone(),
            name: None,
        });
    }
    if let Some(doc) = msg.document() {
        attachments.push(Attachment {
            kind: AttachmentKind::File,
            url: doc.file.id.0.clone(),
            name: doc.file_name.clone(),
        });
    }

    let reply_to = msg.reply_to_message().map(|r| r.id.0);

    Some(ChannelMessage {
        chat_id,
        sender_id,
        content,
        attachments,
        reply_to,
        timestamp: msg.date.timestamp() as u64,
    })
}
