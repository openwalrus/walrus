//! Framing codec tests.

use walrus_protocol::ClientMessage;
use walrus_protocol::codec::{FrameError, read_message, write_message};

#[tokio::test]
async fn codec_roundtrip() {
    let msg = ClientMessage::Send {
        agent: "assistant".into(),
        content: "hello".to_string(),
    };

    let mut buf = Vec::new();
    write_message(&mut buf, &msg).await.unwrap();

    let mut cursor = std::io::Cursor::new(buf);
    let decoded: ClientMessage = read_message(&mut cursor).await.unwrap();

    match decoded {
        ClientMessage::Send { agent, content } => {
            assert_eq!(agent.as_str(), "assistant");
            assert_eq!(content, "hello");
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[tokio::test]
async fn codec_empty_frame() {
    let msg = ClientMessage::Ping;

    let mut buf = Vec::new();
    write_message(&mut buf, &msg).await.unwrap();

    let mut cursor = std::io::Cursor::new(buf);
    let decoded: ClientMessage = read_message(&mut cursor).await.unwrap();
    assert!(matches!(decoded, ClientMessage::Ping));
}

#[tokio::test]
async fn codec_too_large() {
    // Craft a frame header claiming 17 MiB payload.
    let len: u32 = 17 * 1024 * 1024;
    let mut buf = Vec::new();
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(b"{}"); // dummy payload (won't be read)

    let mut cursor = std::io::Cursor::new(buf);
    let result: Result<ClientMessage, _> = read_message(&mut cursor).await;
    assert!(matches!(result, Err(FrameError::TooLarge { .. })));
}

#[tokio::test]
async fn codec_message_roundtrip() {
    use walrus_protocol::ServerMessage;

    let msg = ServerMessage::Error {
        code: 500,
        message: "internal error".to_string(),
    };

    let mut buf = Vec::new();
    write_message(&mut buf, &msg).await.unwrap();

    let mut cursor = std::io::Cursor::new(buf);
    let decoded: ServerMessage = read_message(&mut cursor).await.unwrap();

    match decoded {
        ServerMessage::Error { code, message } => {
            assert_eq!(code, 500);
            assert_eq!(message, "internal error");
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[tokio::test]
async fn codec_connection_closed() {
    let buf: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(buf);
    let result: Result<ClientMessage, _> = read_message(&mut cursor).await;
    assert!(matches!(result, Err(FrameError::ConnectionClosed)));
}
