use bytes::Bytes;
use crabllm_core::{
    AudioSpeechRequest, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse,
    EmbeddingRequest, EmbeddingResponse, Error, ImageRequest, ProviderConfig, ProviderKind,
};
use futures::stream::{BoxStream, StreamExt};
pub use registry::{Deployment, ProviderRegistry};

mod provider;
mod registry;

/// A configured provider instance, ready to dispatch requests.
#[derive(Debug, Clone)]
pub enum Provider {
    /// OpenAI-compatible providers (OpenAI, Ollama, vLLM, Groq, etc.).
    /// Request body is forwarded as-is with URL + auth rewrite.
    Openai { base_url: String, api_key: String },
    /// Anthropic Messages API. Requires request/response translation.
    Anthropic { api_key: String },
    /// Google Gemini API. Requires request/response translation.
    Google { api_key: String },
    /// AWS Bedrock. Requires SigV4 signing + translation.
    Bedrock {
        region: String,
        access_key: String,
        secret_key: String,
    },
    /// Azure OpenAI. Uses deployment-based URL and api-key header.
    Azure {
        base_url: String,
        api_key: String,
        api_version: String,
    },
}

impl From<&ProviderConfig> for Provider {
    fn from(config: &ProviderConfig) -> Self {
        match config.kind {
            ProviderKind::Openai => Provider::Openai {
                base_url: config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                api_key: config.api_key.clone().unwrap_or_default(),
            },
            ProviderKind::Anthropic => Provider::Anthropic {
                api_key: config.api_key.clone().unwrap_or_default(),
            },
            ProviderKind::Google => Provider::Google {
                api_key: config.api_key.clone().unwrap_or_default(),
            },
            ProviderKind::Ollama => Provider::Openai {
                base_url: config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434/v1".to_string()),
                api_key: config.api_key.clone().unwrap_or_default(),
            },
            ProviderKind::Azure => Provider::Azure {
                base_url: config.base_url.clone().unwrap_or_default(),
                api_key: config.api_key.clone().unwrap_or_default(),
                api_version: config
                    .api_version
                    .clone()
                    .unwrap_or_else(|| "2024-02-15-preview".to_string()),
            },
            ProviderKind::Bedrock => Provider::Bedrock {
                region: config.region.clone().unwrap_or_default(),
                access_key: config.access_key.clone().unwrap_or_default(),
                secret_key: config.secret_key.clone().unwrap_or_default(),
            },
            ProviderKind::LlamaCpp => {
                // LlamaCpp providers are constructed after the managed
                // llama-server process starts and a port is known.
                // Use base_url if explicitly set (external llama-server),
                // otherwise this will be overwritten by the process manager.
                Provider::Openai {
                    base_url: config.base_url.clone().unwrap_or_default(),
                    api_key: String::new(),
                }
            }
        }
    }
}

impl Provider {
    /// Send a non-streaming chat completion request.
    pub async fn chat_completion(
        &self,
        client: &reqwest::Client,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Error> {
        match self {
            Provider::Openai { base_url, api_key } => {
                provider::openai::chat_completion(client, base_url, api_key, request).await
            }
            Provider::Anthropic { api_key } => {
                provider::anthropic::chat_completion(client, api_key, request).await
            }
            Provider::Google { api_key } => {
                provider::google::chat_completion(client, api_key, request).await
            }
            #[cfg(feature = "provider-bedrock")]
            Provider::Bedrock {
                region,
                access_key,
                secret_key,
            } => {
                provider::bedrock::chat_completion(client, region, access_key, secret_key, request)
                    .await
            }
            #[cfg(not(feature = "provider-bedrock"))]
            Provider::Bedrock { .. } => Err(provider::bedrock::not_implemented("chat")),
            Provider::Azure {
                base_url,
                api_key,
                api_version,
            } => {
                provider::azure::chat_completion(client, base_url, api_key, api_version, request)
                    .await
            }
        }
    }

    /// Send an embedding request.
    pub async fn embedding(
        &self,
        client: &reqwest::Client,
        request: &EmbeddingRequest,
    ) -> Result<EmbeddingResponse, Error> {
        match self {
            Provider::Openai { base_url, api_key } => {
                provider::openai::embedding(client, base_url, api_key, request).await
            }
            Provider::Anthropic { .. } => Err(provider::anthropic::not_implemented("embedding")),
            Provider::Google { .. } => Err(provider::google::not_implemented("embedding")),
            Provider::Bedrock { .. } => Err(provider::bedrock::not_implemented("embedding")),
            Provider::Azure {
                base_url,
                api_key,
                api_version,
            } => provider::azure::embedding(client, base_url, api_key, api_version, request).await,
        }
    }

    /// Send an image generation request. Returns raw bytes + content-type.
    pub async fn image_generation(
        &self,
        client: &reqwest::Client,
        request: &ImageRequest,
    ) -> Result<(Bytes, String), Error> {
        match self {
            Provider::Openai { base_url, api_key } => {
                provider::openai::image_generation(client, base_url, api_key, request).await
            }
            Provider::Anthropic { .. } => {
                Err(provider::anthropic::not_implemented("image_generation"))
            }
            Provider::Google { .. } => Err(provider::google::not_implemented("image_generation")),
            Provider::Bedrock { .. } => Err(provider::bedrock::not_implemented("image_generation")),
            Provider::Azure {
                base_url,
                api_key,
                api_version,
            } => {
                provider::azure::image_generation(client, base_url, api_key, api_version, request)
                    .await
            }
        }
    }

    /// Send a text-to-speech request. Returns raw audio bytes + content-type.
    pub async fn audio_speech(
        &self,
        client: &reqwest::Client,
        request: &AudioSpeechRequest,
    ) -> Result<(Bytes, String), Error> {
        match self {
            Provider::Openai { base_url, api_key } => {
                provider::openai::audio_speech(client, base_url, api_key, request).await
            }
            Provider::Anthropic { .. } => Err(provider::anthropic::not_implemented("audio_speech")),
            Provider::Google { .. } => Err(provider::google::not_implemented("audio_speech")),
            Provider::Bedrock { .. } => Err(provider::bedrock::not_implemented("audio_speech")),
            Provider::Azure {
                base_url,
                api_key,
                api_version,
            } => {
                provider::azure::audio_speech(client, base_url, api_key, api_version, request).await
            }
        }
    }

    /// Send an audio transcription request. Takes a multipart form + model name.
    /// Returns raw response bytes + content-type.
    pub async fn audio_transcription(
        &self,
        client: &reqwest::Client,
        model: &str,
        form: reqwest::multipart::Form,
    ) -> Result<(Bytes, String), Error> {
        match self {
            Provider::Openai { base_url, api_key } => {
                provider::openai::audio_transcription(client, base_url, api_key, form).await
            }
            Provider::Anthropic { .. } => {
                Err(provider::anthropic::not_implemented("audio_transcription"))
            }
            Provider::Google { .. } => {
                Err(provider::google::not_implemented("audio_transcription"))
            }
            Provider::Bedrock { .. } => {
                Err(provider::bedrock::not_implemented("audio_transcription"))
            }
            Provider::Azure {
                base_url,
                api_key,
                api_version,
            } => {
                provider::azure::audio_transcription(
                    client,
                    base_url,
                    api_key,
                    api_version,
                    model,
                    form,
                )
                .await
            }
        }
    }

    /// Send a streaming chat completion request.
    /// Returns a boxed async stream of parsed SSE chunks.
    pub async fn chat_completion_stream(
        &self,
        client: &reqwest::Client,
        request: &ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<ChatCompletionChunk, Error>>, Error> {
        match self {
            Provider::Openai { base_url, api_key } => {
                let s =
                    provider::openai::chat_completion_stream(client, base_url, api_key, request)
                        .await?;
                Ok(s.boxed())
            }
            Provider::Anthropic { api_key } => {
                let s = provider::anthropic::chat_completion_stream(
                    client,
                    api_key,
                    request,
                    &request.model,
                )
                .await?;
                Ok(s.boxed())
            }
            Provider::Google { api_key } => {
                let s = provider::google::chat_completion_stream(
                    client,
                    api_key,
                    request,
                    &request.model,
                )
                .await?;
                Ok(s.boxed())
            }
            #[cfg(feature = "provider-bedrock")]
            Provider::Bedrock {
                region,
                access_key,
                secret_key,
            } => {
                let s = provider::bedrock::chat_completion_stream(
                    client,
                    region,
                    access_key,
                    secret_key,
                    request,
                    &request.model,
                )
                .await?;
                Ok(s.boxed())
            }
            #[cfg(not(feature = "provider-bedrock"))]
            Provider::Bedrock { .. } => Err(provider::bedrock::not_implemented("streaming")),
            Provider::Azure {
                base_url,
                api_key,
                api_version,
            } => {
                let s = provider::azure::chat_completion_stream(
                    client,
                    base_url,
                    api_key,
                    api_version,
                    request,
                )
                .await?;
                Ok(s.boxed())
            }
        }
    }
}
