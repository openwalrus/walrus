use bytes::{Buf, Bytes, BytesMut};
use crabllm_core::{
    AudioSpeechRequest, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse,
    EmbeddingRequest, EmbeddingResponse, Error, ImageRequest,
};
use futures::stream::{self, Stream};
use reqwest::Response;

/// Send a non-streaming chat completion to an OpenAI-compatible endpoint.
pub async fn chat_completion(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    request: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse, Error> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(request)
        .send()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    resp.json::<ChatCompletionResponse>()
        .await
        .map_err(|e| Error::Internal(e.to_string()))
}

/// Send an embedding request to an OpenAI-compatible endpoint.
pub async fn embedding(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    request: &EmbeddingRequest,
) -> Result<EmbeddingResponse, Error> {
    let url = format!("{}/embeddings", base_url.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(request)
        .send()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    resp.json::<EmbeddingResponse>()
        .await
        .map_err(|e| Error::Internal(e.to_string()))
}

/// Send a streaming chat completion to an OpenAI-compatible endpoint.
/// Returns an async stream of parsed SSE chunks.
pub async fn chat_completion_stream(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    request: &ChatCompletionRequest,
) -> Result<impl Stream<Item = Result<ChatCompletionChunk, Error>> + use<>, Error> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(request)
        .send()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    Ok(sse_stream(resp))
}

/// Send an image generation request to an OpenAI-compatible endpoint.
/// Returns raw response bytes and content-type header.
pub async fn image_generation(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    request: &ImageRequest,
) -> Result<(Bytes, String), Error> {
    let url = format!("{}/images/generations", base_url.trim_end_matches('/'));
    raw_pass_through(client, &url, api_key, request).await
}

/// Send a text-to-speech request to an OpenAI-compatible endpoint.
/// Returns raw audio bytes and content-type header.
pub async fn audio_speech(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    request: &AudioSpeechRequest,
) -> Result<(Bytes, String), Error> {
    let url = format!("{}/audio/speech", base_url.trim_end_matches('/'));
    let (bytes, content_type) = raw_pass_through(client, &url, api_key, request).await?;
    // Default to audio/mpeg if upstream omits Content-Type.
    let content_type = if content_type == "application/json" {
        "audio/mpeg".to_string()
    } else {
        content_type
    };
    Ok((bytes, content_type))
}

/// Forward a JSON request and return raw response bytes + content-type.
pub(crate) async fn raw_pass_through<T: serde::Serialize>(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    request: &T,
) -> Result<(Bytes, String), Error> {
    let resp = client
        .post(url)
        .bearer_auth(api_key)
        .json(request)
        .send()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok((bytes, content_type))
}

/// Send an audio transcription request to an OpenAI-compatible endpoint.
/// Takes a pre-built multipart form. Returns raw response bytes + content-type.
pub async fn audio_transcription(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    form: reqwest::multipart::Form,
) -> Result<(Bytes, String), Error> {
    let url = format!("{}/audio/transcriptions", base_url.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok((bytes, content_type))
}

/// Parse an SSE byte stream into `ChatCompletionChunk` items.
pub(crate) fn sse_stream(resp: Response) -> impl Stream<Item = Result<ChatCompletionChunk, Error>> {
    let byte_stream = resp.bytes_stream();

    stream::unfold(
        (byte_stream, BytesMut::new()),
        |(mut byte_stream, mut buffer)| async move {
            use futures::TryStreamExt;

            loop {
                if let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                    let mut line_end = newline_pos;
                    if line_end > 0 && buffer[line_end - 1] == b'\r' {
                        line_end -= 1;
                    }
                    let line = &buffer[..line_end];

                    if line.is_empty() {
                        buffer.advance(newline_pos + 1);
                        continue;
                    }

                    if let Some(data) = line.strip_prefix(b"data: ") {
                        let data = match std::str::from_utf8(data) {
                            Ok(s) => s.trim(),
                            Err(_) => {
                                buffer.advance(newline_pos + 1);
                                continue;
                            }
                        };
                        if data == "[DONE]" {
                            return None;
                        }
                        let result = match serde_json::from_str::<ChatCompletionChunk>(data) {
                            Ok(chunk) => Ok(chunk),
                            Err(e) => Err(Error::Internal(format!("SSE parse error: {e}"))),
                        };
                        buffer.advance(newline_pos + 1);
                        return Some((result, (byte_stream, buffer)));
                    }
                    // Skip non-data lines (comments, event:, etc.)
                    buffer.advance(newline_pos + 1);
                    continue;
                }

                // Need more data from the stream.
                match byte_stream.try_next().await {
                    Ok(Some(bytes)) => {
                        buffer.extend_from_slice(&bytes);
                    }
                    Ok(None) => return None,
                    Err(e) => {
                        return Some((
                            Err(Error::Internal(format!("stream error: {e}"))),
                            (byte_stream, buffer),
                        ));
                    }
                }
            }
        },
    )
}
