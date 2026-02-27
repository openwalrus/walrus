//! Length-prefixed framing codec for Unix domain socket transport.
//!
//! Wire format: `[u32 BE length][JSON payload]`. The length is the byte count
//! of the JSON payload only (not including the 4-byte header).

use serde::{Serialize, de::DeserializeOwned};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Maximum frame size: 16 MiB.
const MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;

/// Errors that can occur during frame read/write.
#[derive(Debug)]
pub enum FrameError {
    /// Underlying I/O error.
    Io(io::Error),
    /// Frame exceeds the maximum allowed size.
    TooLarge { size: u32 },
    /// JSON serialization/deserialization error.
    Json(serde_json::Error),
    /// The connection was closed (EOF during read).
    ConnectionClosed,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::TooLarge { size } => {
                write!(f, "frame too large: {size} bytes (max {MAX_FRAME_SIZE})")
            }
            Self::Json(e) => write!(f, "json error: {e}"),
            Self::ConnectionClosed => write!(f, "connection closed"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for FrameError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for FrameError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// Write a typed message as a length-prefixed JSON frame.
pub async fn write_message<W, T>(writer: &mut W, msg: &T) -> Result<(), FrameError>
where
    W: tokio::io::AsyncWrite + Unpin,
    T: Serialize,
{
    let data = serde_json::to_vec(msg)?;
    let len = data.len() as u32;
    if len > MAX_FRAME_SIZE {
        return Err(FrameError::TooLarge { size: len });
    }
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&data).await?;
    writer.flush().await?;
    Ok(())
}

/// Read a length-prefixed JSON frame and deserialize into a typed message.
pub async fn read_message<R, T>(reader: &mut R) -> Result<T, FrameError>
where
    R: tokio::io::AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            return Err(FrameError::ConnectionClosed);
        }
        Err(e) => return Err(FrameError::Io(e)),
    }

    let len = u32::from_be_bytes(len_buf);
    if len > MAX_FRAME_SIZE {
        return Err(FrameError::TooLarge { size: len });
    }

    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf).await?;
    let msg = serde_json::from_slice(&buf)?;
    Ok(msg)
}
