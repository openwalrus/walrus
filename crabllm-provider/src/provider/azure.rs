use crate::provider::openai;
use bytes::Bytes;
use crabllm_core::{
    AudioSpeechRequest, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse,
    EmbeddingRequest, EmbeddingResponse, Error, ImageRequest,
};
use futures::stream::Stream;

/// Build an Azure OpenAI deployment URL.
/// Format: {base_url}/openai/deployments/{model}/{path}?api-version={api_version}
fn azure_url(base_url: &str, model: &str, path: &str, api_version: &str) -> String {
    format!(
        "{}/openai/deployments/{}/{}?api-version={}",
        base_url.trim_end_matches('/'),
        model,
        path,
        api_version,
    )
}

/// Send a non-streaming chat completion to an Azure OpenAI deployment.
pub async fn chat_completion(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    api_version: &str,
    request: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse, Error> {
    let url = azure_url(base_url, &request.model, "chat/completions", api_version);
    let resp = client
        .post(&url)
        .header("api-key", api_key)
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

/// Send an embedding request to an Azure OpenAI deployment.
pub async fn embedding(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    api_version: &str,
    request: &EmbeddingRequest,
) -> Result<EmbeddingResponse, Error> {
    let url = azure_url(base_url, &request.model, "embeddings", api_version);
    let resp = client
        .post(&url)
        .header("api-key", api_key)
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

/// Send an image generation request to an Azure OpenAI deployment.
pub async fn image_generation(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    api_version: &str,
    request: &ImageRequest,
) -> Result<(Bytes, String), Error> {
    let url = azure_url(base_url, &request.model, "images/generations", api_version);
    raw_pass_through(client, &url, api_key, request).await
}

/// Send a text-to-speech request to an Azure OpenAI deployment.
pub async fn audio_speech(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    api_version: &str,
    request: &AudioSpeechRequest,
) -> Result<(Bytes, String), Error> {
    let url = azure_url(base_url, &request.model, "audio/speech", api_version);
    let (bytes, content_type) = raw_pass_through(client, &url, api_key, request).await?;
    let content_type = if content_type == "application/json" {
        "audio/mpeg".to_string()
    } else {
        content_type
    };
    Ok((bytes, content_type))
}

/// Forward a JSON request to Azure and return raw response bytes + content-type.
pub(crate) async fn raw_pass_through<T: serde::Serialize>(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    request: &T,
) -> Result<(Bytes, String), Error> {
    let resp = client
        .post(url)
        .header("api-key", api_key)
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

/// Send an audio transcription request to an Azure OpenAI deployment.
/// Takes model separately since the multipart form is opaque.
pub async fn audio_transcription(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    api_version: &str,
    model: &str,
    form: reqwest::multipart::Form,
) -> Result<(Bytes, String), Error> {
    let url = azure_url(base_url, model, "audio/transcriptions", api_version);
    let resp = client
        .post(&url)
        .header("api-key", api_key)
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

/// Send a streaming chat completion to an Azure OpenAI deployment.
/// Reuses the OpenAI SSE parser since Azure streams in the same format.
pub async fn chat_completion_stream(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    api_version: &str,
    request: &ChatCompletionRequest,
) -> Result<impl Stream<Item = Result<ChatCompletionChunk, Error>> + use<>, Error> {
    let url = azure_url(base_url, &request.model, "chat/completions", api_version);
    let resp = client
        .post(&url)
        .header("api-key", api_key)
        .json(request)
        .send()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    Ok(openai::sse_stream(resp))
}
