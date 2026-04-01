use crate::provider::schema;
use bytes::{Buf, BytesMut};
use crabllm_core::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Choice, ChunkChoice, Delta,
    Error, FinishReason, FunctionCall, FunctionCallDelta, Message, Role, ToolCall, ToolCallDelta,
    ToolType, Usage,
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_MAX_TOKENS: u32 = 4096;

// ── Gemini-native request types ──

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiToolDef>>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum GeminiRole {
    User,
    Model,
}

#[derive(Serialize, Deserialize)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<GeminiRole>,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<GeminiFunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_response: Option<GeminiFunctionResponse>,
}

#[derive(Serialize, Deserialize, Clone)]
struct GeminiFunctionCall {
    name: String,
    #[serde(default)]
    args: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct GeminiFunctionResponse {
    name: String,
    response: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiToolDef {
    function_declarations: Vec<GeminiFunctionDecl>,
}

#[derive(Serialize)]
struct GeminiFunctionDecl {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

// ── Gemini-native response types ──

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum GeminiFinishReason {
    Stop,
    MaxTokens,
    Safety,
    Recitation,
    Blocklist,
    ProhibitedContent,
    Spii,
    MalformedFunctionCall,
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    #[serde(default)]
    content: Option<GeminiContent>,
    #[serde(default)]
    finish_reason: Option<GeminiFinishReason>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsage {
    #[serde(default)]
    prompt_token_count: u32,
    #[serde(default)]
    candidates_token_count: u32,
    #[serde(default)]
    total_token_count: u32,
}

// ── Translation ──

fn translate_request(request: &ChatCompletionRequest) -> GeminiRequest {
    // Build tool_call_id → function_name index so Tool-role messages can
    // resolve the function name even when msg.name is None.
    let mut tc_names = std::collections::HashMap::<&str, &str>::new();
    for msg in &request.messages {
        if let Some(tool_calls) = &msg.tool_calls {
            for tc in tool_calls {
                tc_names.insert(&tc.id, &tc.function.name);
            }
        }
    }

    let mut system_parts = Vec::new();
    let mut contents = Vec::new();

    for msg in &request.messages {
        if msg.role == Role::System {
            if let Some(content) = &msg.content
                && let Some(s) = content.as_str()
            {
                system_parts.push(GeminiPart {
                    text: Some(s.to_string()),
                    function_call: None,
                    function_response: None,
                });
            }
        } else if msg.role == Role::Tool {
            // Tool result → user message with functionResponse part.
            // Prefer msg.name, fall back to looking up by tool_call_id.
            let name = msg
                .name
                .clone()
                .or_else(|| {
                    msg.tool_call_id
                        .as_deref()
                        .and_then(|id| tc_names.get(id).map(|n| n.to_string()))
                })
                .unwrap_or_default();
            let response_val = msg
                .content
                .as_ref()
                .and_then(|c| {
                    if c.is_object() || c.is_array() {
                        Some(c.clone())
                    } else if let Some(s) = c.as_str() {
                        serde_json::from_str(s).ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(serde_json::json!({"result": msg.content.as_ref().and_then(|c| c.as_str()).unwrap_or("")}));
            contents.push(GeminiContent {
                role: Some(GeminiRole::User),
                parts: vec![GeminiPart {
                    text: None,
                    function_call: None,
                    function_response: Some(GeminiFunctionResponse {
                        name,
                        response: response_val,
                    }),
                }],
            });
        } else if msg.role == Role::Assistant
            && let Some(tool_calls) = &msg.tool_calls
        {
            // Assistant message with tool_calls → model message with functionCall parts.
            let mut parts = Vec::new();
            if let Some(content) = &msg.content
                && let Some(s) = content.as_str()
                && !s.is_empty()
            {
                parts.push(GeminiPart {
                    text: Some(s.to_string()),
                    function_call: None,
                    function_response: None,
                });
            }
            for tc in tool_calls {
                let args = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                parts.push(GeminiPart {
                    text: None,
                    function_call: Some(GeminiFunctionCall {
                        name: tc.function.name.clone(),
                        args,
                    }),
                    function_response: None,
                });
            }
            contents.push(GeminiContent {
                role: Some(GeminiRole::Model),
                parts,
            });
        } else {
            let role = match msg.role {
                Role::Assistant => GeminiRole::Model,
                _ => GeminiRole::User,
            };
            let text = msg
                .content
                .as_ref()
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            contents.push(GeminiContent {
                role: Some(role),
                parts: vec![GeminiPart {
                    text: Some(text),
                    function_call: None,
                    function_response: None,
                }],
            });
        }
    }

    let system_instruction = if system_parts.is_empty() {
        None
    } else {
        Some(GeminiContent {
            role: None,
            parts: system_parts,
        })
    };

    let stop_sequences = request.stop.as_ref().map(|s| match s {
        crabllm_core::Stop::Single(s) => vec![s.clone()],
        crabllm_core::Stop::Multiple(v) => v.clone(),
    });

    let generation_config = Some(GenerationConfig {
        max_output_tokens: Some(request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
        temperature: request.temperature,
        top_p: request.top_p,
        stop_sequences,
    });

    let tools = request.tools.as_ref().map(|tools| {
        vec![GeminiToolDef {
            function_declarations: tools
                .iter()
                .map(|t| GeminiFunctionDecl {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    parameters: t.function.parameters.clone().map(|mut p| {
                        schema::inline_refs(&mut p);
                        schema::strip_schema_meta(&mut p);
                        schema::flatten_nullable(&mut p);
                        schema::strip_fields(
                            &mut p,
                            &[
                                "title",
                                "default",
                                "examples",
                                "$comment",
                                "additionalProperties",
                            ],
                        );
                        p
                    }),
                })
                .collect(),
        }]
    });

    GeminiRequest {
        contents,
        system_instruction,
        generation_config,
        tools,
    }
}

impl From<&GeminiFinishReason> for FinishReason {
    fn from(r: &GeminiFinishReason) -> Self {
        match r {
            GeminiFinishReason::Stop => FinishReason::Stop,
            GeminiFinishReason::MaxTokens => FinishReason::Length,
            GeminiFinishReason::Safety
            | GeminiFinishReason::Blocklist
            | GeminiFinishReason::ProhibitedContent
            | GeminiFinishReason::Spii => FinishReason::ContentFilter,
            GeminiFinishReason::Recitation => FinishReason::Custom("recitation".into()),
            GeminiFinishReason::MalformedFunctionCall => {
                FinishReason::Custom("malformed_function_call".into())
            }
            GeminiFinishReason::Other => FinishReason::Custom("other".into()),
        }
    }
}

impl From<GeminiUsage> for Usage {
    fn from(u: GeminiUsage) -> Self {
        Usage {
            prompt_tokens: u.prompt_token_count,
            completion_tokens: u.candidates_token_count,
            total_tokens: u.total_token_count,
            completion_tokens_details: None,
            prompt_cache_hit_tokens: None,
            prompt_cache_miss_tokens: None,
        }
    }
}

/// Extract text and tool calls from response candidate parts.
fn extract_parts(candidate: &GeminiCandidate) -> (String, Vec<ToolCall>) {
    let mut text = String::new();
    let mut tool_calls = Vec::new();

    if let Some(content) = &candidate.content {
        for (i, part) in content.parts.iter().enumerate() {
            if let Some(t) = &part.text {
                text.push_str(t);
            }
            if let Some(fc) = &part.function_call {
                tool_calls.push(ToolCall {
                    index: None,
                    id: format!("call_{i}"),
                    kind: ToolType::Function,
                    function: FunctionCall {
                        name: fc.name.clone(),
                        arguments: serde_json::to_string(&fc.args).unwrap_or_default(),
                    },
                });
            }
        }
    }

    (text, tool_calls)
}

fn translate_response(resp: GeminiResponse, model: &str) -> ChatCompletionResponse {
    let (content_text, tool_calls, finish_reason) = resp
        .candidates
        .first()
        .map(|c| {
            let (text, tcs) = extract_parts(c);
            (text, tcs, c.finish_reason.as_ref().map(Into::into))
        })
        .unwrap_or_default();

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
            finish_reason,
            logprobs: None,
        }],
        usage: resp.usage_metadata.map(Usage::from),
        system_fingerprint: None,
    }
}

pub fn not_implemented(name: &str) -> Error {
    Error::Internal(format!("google {name} not supported"))
}

// ── Public API ──

pub async fn chat_completion(
    client: &reqwest::Client,
    api_key: &str,
    request: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse, Error> {
    let gemini_req = translate_request(request);
    let url = format!("{}/models/{}:generateContent", BASE_URL, request.model);

    let resp = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .header("content-type", "application/json")
        .json(&gemini_req)
        .send()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    let gemini_resp: GeminiResponse = resp
        .json()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    Ok(translate_response(gemini_resp, &request.model))
}

pub async fn chat_completion_stream(
    client: &reqwest::Client,
    api_key: &str,
    request: &ChatCompletionRequest,
    model: &str,
) -> Result<impl Stream<Item = Result<ChatCompletionChunk, Error>> + use<>, Error> {
    let gemini_req = translate_request(request);
    let url = format!(
        "{}/models/{}:streamGenerateContent?alt=sse",
        BASE_URL, request.model
    );

    let resp = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .header("content-type", "application/json")
        .json(&gemini_req)
        .send()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    let model = model.to_string();
    Ok(gemini_sse_stream(resp, model))
}

fn gemini_sse_stream(
    resp: reqwest::Response,
    model: String,
) -> impl Stream<Item = Result<ChatCompletionChunk, Error>> {
    let byte_stream = resp.bytes_stream();

    stream::unfold(
        (byte_stream, BytesMut::new(), model, 0u64),
        |(mut byte_stream, mut buffer, model, mut chunk_idx)| async move {
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

                    let Some(data) = line.strip_prefix(b"data: ") else {
                        buffer.advance(newline_pos + 1);
                        continue;
                    };
                    let data = match std::str::from_utf8(data) {
                        Ok(s) => s.trim(),
                        Err(_) => {
                            buffer.advance(newline_pos + 1);
                            continue;
                        }
                    };

                    let gemini_resp: GeminiResponse = match serde_json::from_str(data) {
                        Ok(r) => r,
                        Err(_) => {
                            buffer.advance(newline_pos + 1);
                            continue;
                        }
                    };

                    let candidate = match gemini_resp.candidates.first() {
                        Some(c) => c,
                        None => {
                            buffer.advance(newline_pos + 1);
                            continue;
                        }
                    };

                    let (text, tool_calls) = extract_parts(candidate);
                    let finish_reason = candidate.finish_reason.as_ref().map(Into::into);

                    let has_text = !text.is_empty();
                    let has_tools = !tool_calls.is_empty();

                    if !has_text && !has_tools && finish_reason.is_none() {
                        buffer.advance(newline_pos + 1);
                        continue;
                    }

                    buffer.advance(newline_pos + 1);

                    chunk_idx += 1;
                    let tool_call_deltas = if has_tools {
                        Some(
                            tool_calls
                                .into_iter()
                                .enumerate()
                                .map(|(i, tc)| ToolCallDelta {
                                    index: i as u32,
                                    id: Some(tc.id),
                                    kind: Some(ToolType::Function),
                                    function: Some(FunctionCallDelta {
                                        name: Some(tc.function.name),
                                        arguments: Some(tc.function.arguments),
                                    }),
                                })
                                .collect(),
                        )
                    } else {
                        None
                    };

                    let chunk = ChatCompletionChunk {
                        id: format!("chatcmpl-{chunk_idx}"),
                        object: "chat.completion.chunk".to_string(),
                        created: 0,
                        model: model.clone(),
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: Delta {
                                role: if chunk_idx == 1 {
                                    Some(Role::Assistant)
                                } else {
                                    None
                                },
                                content: if has_text { Some(text) } else { None },
                                tool_calls: tool_call_deltas,
                                reasoning_content: None,
                            },
                            finish_reason,
                            logprobs: None,
                        }],
                        usage: gemini_resp.usage_metadata.map(Usage::from),
                        system_fingerprint: None,
                    };
                    return Some((Ok(chunk), (byte_stream, buffer, model, chunk_idx)));
                }

                match byte_stream.try_next().await {
                    Ok(Some(bytes)) => {
                        buffer.extend_from_slice(&bytes);
                    }
                    Ok(None) => return None,
                    Err(e) => {
                        return Some((
                            Err(Error::Internal(format!("stream error: {e}"))),
                            (byte_stream, buffer, model, chunk_idx),
                        ));
                    }
                }
            }
        },
    )
}
