//! Tests for Channel types.

use walrus_core::{AttachmentKind, ChannelMessage, Platform};

#[test]
fn platform_enum_variants() {
    let p = Platform::Telegram;
    assert_eq!(p, Platform::Telegram);
}

#[test]
fn channel_message_to_llm_message() {
    let msg = ChannelMessage {
        platform: Platform::Telegram,
        channel_id: "chat123".into(),
        sender_id: "user456".into(),
        content: "hello agent".into(),
        attachments: vec![],
        reply_to: None,
        timestamp: 1000,
    };
    let llm_msg: llm::Message = msg.into();
    assert_eq!(llm_msg.content, "hello agent");
    assert_eq!(llm_msg.role, llm::Role::User);
}

#[test]
fn attachment_kind_variants() {
    let kinds = [
        AttachmentKind::Image,
        AttachmentKind::File,
        AttachmentKind::Audio,
        AttachmentKind::Video,
    ];
    assert_eq!(kinds.len(), 4);
    assert_ne!(AttachmentKind::Image, AttachmentKind::File);
}
