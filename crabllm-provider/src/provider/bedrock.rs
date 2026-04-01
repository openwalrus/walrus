use crabllm_core::Error;

pub fn not_implemented(name: &str) -> Error {
    Error::Internal(format!("bedrock {name} not yet implemented"))
}

#[cfg(feature = "provider-bedrock")]
pub(crate) use self::sigv4::sign_request;

// ── Converse API (feature-gated) ──

#[cfg(feature = "provider-bedrock")]
use crate::provider::schema;
#[cfg(feature = "provider-bedrock")]
use crabllm_core::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Choice, ChunkChoice, Delta,
    FinishReason, FunctionCall, FunctionCallDelta, Message, Role, ToolCall, ToolCallDelta,
    ToolType, Usage,
};
#[cfg(feature = "provider-bedrock")]
use futures::stream::{self, Stream};
#[cfg(feature = "provider-bedrock")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "provider-bedrock")]
const BASE_URL: &str = "https://bedrock-runtime";
#[cfg(feature = "provider-bedrock")]
const DEFAULT_MAX_TOKENS: u32 = 4096;

// ── Bedrock Converse request types ──

#[cfg(feature = "provider-bedrock")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConverseRequest {
    messages: Vec<ConverseMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Vec<SystemBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inference_config: Option<InferenceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_config: Option<ToolConfig>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Serialize)]
struct ConverseMessage {
    role: String,
    content: Vec<ContentBlock>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ContentBlock {
    Text(String),
    ToolUse {
        tool_use_id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<ToolResultContent>,
    },
}

#[cfg(feature = "provider-bedrock")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ToolResultContent {
    Text(String),
}

#[cfg(feature = "provider-bedrock")]
#[derive(Serialize)]
struct SystemBlock {
    text: String,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InferenceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolConfig {
    tools: Vec<ToolDef>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolDef {
    tool_spec: ToolSpec,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolSpec {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    input_schema: InputSchema,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Serialize)]
struct InputSchema {
    json: serde_json::Value,
}

// ── Bedrock Converse response types ──

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConverseResponse {
    output: ConverseOutput,
    stop_reason: Option<String>,
    usage: Option<ConverseUsage>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
struct ConverseOutput {
    message: Option<ConverseOutputMessage>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
struct ConverseOutputMessage {
    content: Vec<ContentBlock>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConverseUsage {
    input_tokens: u32,
    output_tokens: u32,
    total_tokens: u32,
}

// ── Translation ──

#[cfg(feature = "provider-bedrock")]
fn translate_request(request: &ChatCompletionRequest) -> ConverseRequest {
    let mut system_blocks = Vec::new();
    let mut messages = Vec::new();

    for msg in &request.messages {
        if msg.role == Role::System {
            if let Some(content) = &msg.content
                && let Some(s) = content.as_str()
            {
                system_blocks.push(SystemBlock {
                    text: s.to_string(),
                });
            }
        } else if msg.role == Role::Tool {
            // Tool result → user message with toolResult content block.
            let content_str = msg
                .content
                .as_ref()
                .map(|c| {
                    if let Some(s) = c.as_str() {
                        s.to_string()
                    } else {
                        c.to_string()
                    }
                })
                .unwrap_or_default();
            let tool_use_id = msg.tool_call_id.clone().unwrap_or_default();
            messages.push(ConverseMessage {
                role: "user".to_string(),
                content: vec![ContentBlock::ToolResult {
                    tool_use_id,
                    content: vec![ToolResultContent::Text(content_str)],
                }],
            });
        } else if msg.role == Role::Assistant
            && let Some(tool_calls) = &msg.tool_calls
        {
            // Assistant message with tool_calls → content blocks.
            let mut blocks = Vec::new();
            if let Some(content) = &msg.content
                && let Some(s) = content.as_str()
                && !s.is_empty()
            {
                blocks.push(ContentBlock::Text(s.to_string()));
            }
            for tc in tool_calls {
                let input = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                blocks.push(ContentBlock::ToolUse {
                    tool_use_id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    input,
                });
            }
            messages.push(ConverseMessage {
                role: "assistant".to_string(),
                content: blocks,
            });
        } else {
            let text = msg
                .content
                .as_ref()
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            messages.push(ConverseMessage {
                role: msg.role.as_str().to_string(),
                content: vec![ContentBlock::Text(text)],
            });
        }
    }

    let system = if system_blocks.is_empty() {
        None
    } else {
        Some(system_blocks)
    };

    let stop_sequences = request.stop.as_ref().map(|s| match s {
        crabllm_core::Stop::Single(s) => vec![s.clone()],
        crabllm_core::Stop::Multiple(v) => v.clone(),
    });

    let inference_config = Some(InferenceConfig {
        max_tokens: Some(request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
        temperature: request.temperature,
        top_p: request.top_p,
        stop_sequences,
    });

    let tool_config = request.tools.as_ref().map(|tools| ToolConfig {
        tools: tools
            .iter()
            .map(|t| ToolDef {
                tool_spec: ToolSpec {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    input_schema: InputSchema {
                        json: {
                            let mut s = t
                                .function
                                .parameters
                                .clone()
                                .unwrap_or(serde_json::json!({"type": "object"}));
                            schema::inline_refs(&mut s);
                            s
                        },
                    },
                },
            })
            .collect(),
    });

    ConverseRequest {
        messages,
        system,
        inference_config,
        tool_config,
    }
}

#[cfg(feature = "provider-bedrock")]
fn map_stop_reason(stop_reason: &Option<String>) -> Option<FinishReason> {
    stop_reason.as_ref().map(|r| match r.as_str() {
        "end_turn" | "stop_sequence" => FinishReason::Stop,
        "max_tokens" => FinishReason::Length,
        "tool_use" => FinishReason::ToolCalls,
        other => FinishReason::Custom(other.to_string()),
    })
}

#[cfg(feature = "provider-bedrock")]
fn translate_response(resp: ConverseResponse, model: &str) -> ChatCompletionResponse {
    let mut content_text = String::new();
    let mut tool_calls = Vec::new();

    if let Some(message) = &resp.output.message {
        for block in &message.content {
            match block {
                ContentBlock::Text(t) => content_text.push_str(t),
                ContentBlock::ToolUse {
                    tool_use_id,
                    name,
                    input,
                } => {
                    tool_calls.push(ToolCall {
                        index: None,
                        id: tool_use_id.clone(),
                        kind: ToolType::Function,
                        function: FunctionCall {
                            name: name.clone(),
                            arguments: serde_json::to_string(input).unwrap_or_default(),
                        },
                    });
                }
                ContentBlock::ToolResult { .. } => {}
            }
        }
    }

    let tool_calls_opt = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    let content = if content_text.is_empty() && tool_calls_opt.is_some() {
        None
    } else {
        Some(serde_json::Value::String(content_text))
    };

    ChatCompletionResponse {
        id: String::new(),
        object: "chat.completion".to_string(),
        created: 0,
        model: model.to_string(),
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content,
                tool_calls: tool_calls_opt,
                tool_call_id: None,
                name: None,
                reasoning_content: None,
                extra: Default::default(),
            },
            finish_reason: map_stop_reason(&resp.stop_reason),
            logprobs: None,
        }],
        usage: resp.usage.map(|u| Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.total_tokens,
            completion_tokens_details: None,
            prompt_cache_hit_tokens: None,
            prompt_cache_miss_tokens: None,
        }),
        system_fingerprint: None,
    }
}

// ── Public API ──

#[cfg(feature = "provider-bedrock")]
pub async fn chat_completion(
    client: &reqwest::Client,
    region: &str,
    access_key: &str,
    secret_key: &str,
    request: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse, Error> {
    let bedrock_req = translate_request(request);
    let body = serde_json::to_vec(&bedrock_req).map_err(|e| Error::Internal(e.to_string()))?;
    let url = format!(
        "{BASE_URL}.{region}.amazonaws.com/model/{}/converse",
        request.model
    );

    let req = sign_request(client, "POST", &url, &body, region, access_key, secret_key)?;
    let resp = client
        .execute(req)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    let bedrock_resp: ConverseResponse = resp
        .json()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    Ok(translate_response(bedrock_resp, &request.model))
}

// ── Streaming types ──

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamEvent {
    #[serde(default)]
    message_start: Option<MessageStartEvent>,
    #[serde(default)]
    content_block_start: Option<ContentBlockStartEvent>,
    #[serde(default)]
    content_block_delta: Option<ContentBlockDeltaEvent>,
    #[serde(default)]
    message_stop: Option<MessageStopEvent>,
    #[serde(default)]
    metadata: Option<MetadataEvent>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
struct MessageStartEvent {
    role: Role,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentBlockStartEvent {
    content_block_index: u32,
    start: Option<BlockStart>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockStart {
    #[serde(default)]
    tool_use_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentBlockDeltaEvent {
    #[allow(dead_code)]
    content_block_index: u32,
    delta: Option<BlockDelta>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockDelta {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    tool_use: Option<ToolUseDelta>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
struct ToolUseDelta {
    input: String,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageStopEvent {
    stop_reason: Option<String>,
}

#[cfg(feature = "provider-bedrock")]
#[derive(Deserialize)]
struct MetadataEvent {
    usage: Option<ConverseUsage>,
}

// ── Event-stream binary frame parser ──

#[cfg(feature = "provider-bedrock")]
fn parse_event_payload(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    // AWS event-stream: prelude is 12 bytes (total_len u32be, headers_len u32be, prelude_crc u32be).
    if buf.len() < 12 {
        return None;
    }

    let total_len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if buf.len() < total_len {
        return None;
    }

    let headers_len = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;
    // Payload starts after prelude (12 bytes) + headers.
    let payload_start = 12 + headers_len;
    // Payload ends 4 bytes before message end (message CRC).
    let payload_end = total_len - 4;

    let payload = if payload_start < payload_end {
        buf[payload_start..payload_end].to_vec()
    } else {
        Vec::new()
    };

    // Consume the frame from the buffer.
    buf.drain(..total_len);
    Some(payload)
}

// ── Streaming public API ──

#[cfg(feature = "provider-bedrock")]
pub async fn chat_completion_stream(
    client: &reqwest::Client,
    region: &str,
    access_key: &str,
    secret_key: &str,
    request: &ChatCompletionRequest,
    model: &str,
) -> Result<impl Stream<Item = Result<ChatCompletionChunk, Error>> + use<>, Error> {
    let bedrock_req = translate_request(request);
    let body = serde_json::to_vec(&bedrock_req).map_err(|e| Error::Internal(e.to_string()))?;
    let url = format!(
        "{BASE_URL}.{region}.amazonaws.com/model/{}/converse-stream",
        request.model
    );

    let req = sign_request(client, "POST", &url, &body, region, access_key, secret_key)?;
    let resp = client
        .execute(req)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    let model = model.to_string();
    Ok(bedrock_event_stream(resp, model))
}

#[cfg(feature = "provider-bedrock")]
struct StreamState {
    chunk_idx: u64,
    tool_call_idx: u32,
}

#[cfg(feature = "provider-bedrock")]
fn bedrock_event_stream(
    resp: reqwest::Response,
    model: String,
) -> impl Stream<Item = Result<ChatCompletionChunk, Error>> {
    let byte_stream = resp.bytes_stream();
    let state = StreamState {
        chunk_idx: 0,
        tool_call_idx: 0,
    };

    stream::unfold(
        (byte_stream, Vec::<u8>::new(), model, state),
        |(mut byte_stream, mut buf, model, mut state)| async move {
            use futures::TryStreamExt;

            loop {
                if let Some(payload) = parse_event_payload(&mut buf) {
                    if payload.is_empty() {
                        continue;
                    }

                    let event: StreamEvent = match serde_json::from_slice(&payload) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    // messageStart → emit role chunk.
                    if let Some(ms) = event.message_start {
                        state.chunk_idx += 1;
                        let chunk = ChatCompletionChunk {
                            id: format!("chatcmpl-{}", state.chunk_idx),
                            object: "chat.completion.chunk".to_string(),
                            created: 0,
                            model: model.clone(),
                            choices: vec![ChunkChoice {
                                index: 0,
                                delta: Delta {
                                    role: Some(ms.role),
                                    content: None,
                                    tool_calls: None,
                                    reasoning_content: None,
                                },
                                finish_reason: None,
                                logprobs: None,
                            }],
                            usage: None,
                            system_fingerprint: None,
                        };
                        return Some((Ok(chunk), (byte_stream, buf, model, state)));
                    }

                    // contentBlockStart with toolUse → emit initial tool call delta.
                    if let Some(cbs) = event.content_block_start {
                        if let Some(start) = &cbs.start
                            && start.tool_use_id.is_some()
                        {
                            state.chunk_idx += 1;
                            let tool_idx = state.tool_call_idx;
                            state.tool_call_idx += 1;
                            let chunk = ChatCompletionChunk {
                                id: format!("chatcmpl-{}", state.chunk_idx),
                                object: "chat.completion.chunk".to_string(),
                                created: 0,
                                model: model.clone(),
                                choices: vec![ChunkChoice {
                                    index: 0,
                                    delta: Delta {
                                        role: None,
                                        content: None,
                                        tool_calls: Some(vec![ToolCallDelta {
                                            index: tool_idx,
                                            id: start.tool_use_id.clone(),
                                            kind: Some(ToolType::Function),
                                            function: Some(FunctionCallDelta {
                                                name: start.name.clone(),
                                                arguments: Some(String::new()),
                                            }),
                                        }]),
                                        reasoning_content: None,
                                    },
                                    finish_reason: None,
                                    logprobs: None,
                                }],
                                usage: None,
                                system_fingerprint: None,
                            };
                            return Some((Ok(chunk), (byte_stream, buf, model, state)));
                        }
                        // Text block start — nothing to emit, wait for deltas.
                        let _ = cbs.content_block_index;
                        continue;
                    }

                    // contentBlockDelta → text or tool input.
                    if let Some(cbd) = event.content_block_delta {
                        if let Some(delta) = &cbd.delta {
                            if let Some(text) = &delta.text {
                                state.chunk_idx += 1;
                                let chunk = ChatCompletionChunk {
                                    id: format!("chatcmpl-{}", state.chunk_idx),
                                    object: "chat.completion.chunk".to_string(),
                                    created: 0,
                                    model: model.clone(),
                                    choices: vec![ChunkChoice {
                                        index: 0,
                                        delta: Delta {
                                            role: None,
                                            content: Some(text.clone()),
                                            tool_calls: None,
                                            reasoning_content: None,
                                        },
                                        finish_reason: None,
                                        logprobs: None,
                                    }],
                                    usage: None,
                                    system_fingerprint: None,
                                };
                                return Some((Ok(chunk), (byte_stream, buf, model, state)));
                            }
                            if let Some(tu) = &delta.tool_use {
                                state.chunk_idx += 1;
                                let tool_idx = state.tool_call_idx.saturating_sub(1);
                                let chunk = ChatCompletionChunk {
                                    id: format!("chatcmpl-{}", state.chunk_idx),
                                    object: "chat.completion.chunk".to_string(),
                                    created: 0,
                                    model: model.clone(),
                                    choices: vec![ChunkChoice {
                                        index: 0,
                                        delta: Delta {
                                            role: None,
                                            content: None,
                                            tool_calls: Some(vec![ToolCallDelta {
                                                index: tool_idx,
                                                id: None,
                                                kind: None,
                                                function: Some(FunctionCallDelta {
                                                    name: None,
                                                    arguments: Some(tu.input.clone()),
                                                }),
                                            }]),
                                            reasoning_content: None,
                                        },
                                        finish_reason: None,
                                        logprobs: None,
                                    }],
                                    usage: None,
                                    system_fingerprint: None,
                                };
                                return Some((Ok(chunk), (byte_stream, buf, model, state)));
                            }
                        }
                        continue;
                    }

                    // messageStop → emit finish reason.
                    if let Some(ms) = event.message_stop {
                        state.chunk_idx += 1;
                        let chunk = ChatCompletionChunk {
                            id: format!("chatcmpl-{}", state.chunk_idx),
                            object: "chat.completion.chunk".to_string(),
                            created: 0,
                            model: model.clone(),
                            choices: vec![ChunkChoice {
                                index: 0,
                                delta: Delta {
                                    role: None,
                                    content: None,
                                    tool_calls: None,
                                    reasoning_content: None,
                                },
                                finish_reason: map_stop_reason(&ms.stop_reason),
                                logprobs: None,
                            }],
                            usage: None,
                            system_fingerprint: None,
                        };
                        return Some((Ok(chunk), (byte_stream, buf, model, state)));
                    }

                    // metadata → emit usage, then end stream.
                    if let Some(meta) = event.metadata {
                        if let Some(u) = meta.usage {
                            state.chunk_idx += 1;
                            let chunk = ChatCompletionChunk {
                                id: format!("chatcmpl-{}", state.chunk_idx),
                                object: "chat.completion.chunk".to_string(),
                                created: 0,
                                model: model.clone(),
                                choices: vec![],
                                usage: Some(Usage {
                                    prompt_tokens: u.input_tokens,
                                    completion_tokens: u.output_tokens,
                                    total_tokens: u.total_tokens,
                                    completion_tokens_details: None,
                                    prompt_cache_hit_tokens: None,
                                    prompt_cache_miss_tokens: None,
                                }),
                                system_fingerprint: None,
                            };
                            return Some((Ok(chunk), (byte_stream, buf, model, state)));
                        }
                        return None;
                    }

                    continue;
                }

                // Need more data from the wire.
                match byte_stream.try_next().await {
                    Ok(Some(bytes)) => buf.extend_from_slice(&bytes),
                    Ok(None) => return None,
                    Err(e) => {
                        return Some((
                            Err(Error::Internal(format!("stream error: {e}"))),
                            (byte_stream, buf, model, state),
                        ));
                    }
                }
            }
        },
    )
}

// ── SigV4 signing ──

#[cfg(feature = "provider-bedrock")]
#[allow(dead_code)]
mod sigv4 {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    use std::{
        fmt::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    const SERVICE: &str = "bedrock-runtime";

    /// AWS SigV4-signed request builder. Constructs a reqwest::Request with
    /// Authorization, x-amz-date, x-amz-content-sha256, and host headers.
    pub fn sign_request(
        client: &reqwest::Client,
        method: &str,
        url: &str,
        body: &[u8],
        region: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<reqwest::Request, crabllm_core::Error> {
        let parsed = reqwest::Url::parse(url)
            .map_err(|e| crabllm_core::Error::Internal(format!("bad url: {e}")))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| crabllm_core::Error::Internal("url has no host".to_string()))?;
        let path = parsed.path();
        let query = parsed.query().unwrap_or("");

        let now = now_utc();
        let date_stamp = &now[..8]; // YYYYMMDD
        let amz_date = &now; // YYYYMMDDTHHMMSSZ

        let content_hash = hex_sha256(body);
        let credential_scope = format!("{date_stamp}/{region}/{SERVICE}/aws4_request");

        // Canonical headers (must be sorted).
        let canonical_headers = format!(
            "content-type:application/json\nhost:{host}\nx-amz-content-sha256:{content_hash}\nx-amz-date:{amz_date}\n"
        );
        let signed_headers = "content-type;host;x-amz-content-sha256;x-amz-date";

        // Canonical request.
        let canonical_request = format!(
            "{method}\n{path}\n{query}\n{canonical_headers}\n{signed_headers}\n{content_hash}"
        );
        let canonical_request_hash = hex_sha256(canonical_request.as_bytes());

        // String to sign.
        let string_to_sign =
            format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{canonical_request_hash}");

        // Signing key derivation.
        let signing_key = derive_signing_key(secret_key, date_stamp, region);
        let signature = hex_hmac_sha256(&signing_key, string_to_sign.as_bytes());

        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={access_key}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
        );

        let req = client
            .request(
                method
                    .parse()
                    .map_err(|e| crabllm_core::Error::Internal(format!("bad method: {e}")))?,
                url,
            )
            .header("content-type", "application/json")
            .header("host", host)
            .header("x-amz-date", amz_date)
            .header("x-amz-content-sha256", &content_hash)
            .header("authorization", &authorization)
            .body(body.to_vec())
            .build()
            .map_err(|e| crabllm_core::Error::Internal(format!("build request: {e}")))?;

        Ok(req)
    }

    fn hex_sha256(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex_encode(&hasher.finalize())
    }

    fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    fn hex_hmac_sha256(key: &[u8], data: &[u8]) -> String {
        hex_encode(&hmac_sha256(key, data))
    }

    fn derive_signing_key(secret_key: &str, date_stamp: &str, region: &str) -> Vec<u8> {
        let k_date = hmac_sha256(
            format!("AWS4{secret_key}").as_bytes(),
            date_stamp.as_bytes(),
        );
        let k_region = hmac_sha256(&k_date, region.as_bytes());
        let k_service = hmac_sha256(&k_region, SERVICE.as_bytes());
        hmac_sha256(&k_service, b"aws4_request")
    }

    fn hex_encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            write!(s, "{b:02x}").unwrap();
        }
        s
    }

    fn now_utc() -> String {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_secs();

        // Convert unix timestamp to YYYYMMDDTHHMMSSZ.
        let days = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;
        let seconds = time_of_day % 60;

        let (year, month, day) = civil_from_days(days as i64);

        format!("{year:04}{month:02}{day:02}T{hours:02}{minutes:02}{seconds:02}Z")
    }

    /// Convert days since 1970-01-01 to (year, month, day).
    /// Algorithm from Howard Hinnant's date library.
    fn civil_from_days(days: i64) -> (i32, u32, u32) {
        let z = days + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = (z - era * 146097) as u32;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        (y as i32, m, d)
    }
}
