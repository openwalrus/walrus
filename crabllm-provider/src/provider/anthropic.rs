use crate::provider::schema;
use bytes::{Buf, BytesMut};
use crabllm_core::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Choice, ChunkChoice, Delta,
    Error, FinishReason, FunctionCall, FunctionCallDelta, Message, Role, Stop, ToolCall,
    ToolCallDelta, ToolChoice, ToolType, Usage,
};
use futures::{
    TryStreamExt,
    stream::{self, Stream},
};
use serde::{Deserialize, Serialize};

const DEFAULT_MAX_TOKENS: u32 = 4096;
const BASE_URL: &str = "https://api.anthropic.com/v1";

// ── Anthropic-native request types ──

#[derive(Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    kind: String,
    budget_tokens: u32,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

/// Message content: either a plain string or an array of content blocks.
#[derive(Serialize)]
#[serde(untagged)]
enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

/// A content block in a message (text, image, tool_use, or tool_result).
#[derive(Serialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: serde_json::Value },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize)]
struct AnthropicTool {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    input_schema: serde_json::Value,
}

// ── Anthropic-native response types ──

#[derive(Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<ResponseContentBlock>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct ResponseContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
    #[serde(default)]
    cache_read_input_tokens: Option<u32>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u32>,
}

// ── Anthropic SSE event types ──

#[derive(Deserialize)]
struct SseEvent {
    #[serde(rename = "type")]
    kind: String,
    #[allow(dead_code)]
    #[serde(default)]
    index: Option<u32>,
    #[serde(default)]
    delta: Option<SseDelta>,
    #[serde(default)]
    content_block: Option<SseContentBlock>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
    #[serde(default)]
    message: Option<SseMessage>,
    #[serde(default)]
    error: Option<SseError>,
}

#[derive(Deserialize)]
struct SseMessage {
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct SseError {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct SseDelta {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    partial_json: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    thinking: Option<String>,
}

#[derive(Deserialize)]
struct SseContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

// ── Translation ──

fn translate_request(request: &ChatCompletionRequest) -> AnthropicRequest {
    let mut system_parts = Vec::new();
    let mut messages = Vec::new();

    for msg in &request.messages {
        if msg.role == Role::System {
            if let Some(content) = &msg.content
                && let Some(s) = content.as_str()
            {
                system_parts.push(s.to_string());
            }
        } else if msg.role == Role::Tool {
            // Tool result → user message with tool_result content block.
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
            messages.push(AnthropicMessage {
                role: "user".to_string(),
                content: AnthropicContent::Blocks(vec![AnthropicContentBlock::ToolResult {
                    tool_use_id,
                    content: content_str,
                }]),
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
                blocks.push(AnthropicContentBlock::Text {
                    text: s.to_string(),
                });
            }
            for tc in tool_calls {
                let input = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                blocks.push(AnthropicContentBlock::ToolUse {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    input,
                });
            }
            messages.push(AnthropicMessage {
                role: "assistant".to_string(),
                content: AnthropicContent::Blocks(blocks),
            });
        } else {
            let anthropic_content = match msg.content.as_ref() {
                Some(c) if c.is_array() => {
                    let mut blocks = Vec::new();
                    if let Some(parts) = c.as_array() {
                        for part in parts {
                            match part.get("type").and_then(|t| t.as_str()) {
                                Some("text") => {
                                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                        blocks.push(AnthropicContentBlock::Text {
                                            text: text.to_string(),
                                        });
                                    }
                                }
                                Some("image_url") => {
                                    if let Some(url) = part
                                        .get("image_url")
                                        .and_then(|iu| iu.get("url"))
                                        .and_then(|u| u.as_str())
                                    {
                                        let source = if let Some(rest) = url.strip_prefix("data:") {
                                            // data:image/png;base64,<data>
                                            let (meta, data) = rest.split_once(',').unwrap_or((
                                                "application/octet-stream;base64",
                                                rest,
                                            ));
                                            let media_type =
                                                meta.strip_suffix(";base64").unwrap_or(meta);
                                            serde_json::json!({
                                                "type": "base64",
                                                "media_type": media_type,
                                                "data": data,
                                            })
                                        } else {
                                            serde_json::json!({
                                                "type": "url",
                                                "url": url,
                                            })
                                        };
                                        blocks.push(AnthropicContentBlock::Image { source });
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    if blocks.is_empty() {
                        AnthropicContent::Text(String::new())
                    } else {
                        AnthropicContent::Blocks(blocks)
                    }
                }
                Some(c) => AnthropicContent::Text(c.as_str().unwrap_or("").to_string()),
                None => AnthropicContent::Text(String::new()),
            };
            messages.push(AnthropicMessage {
                role: msg.role.as_str().to_string(),
                content: anthropic_content,
            });
        }
    }

    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n"))
    };

    // B2: When tool_choice is "none", omit tools and tool_choice entirely.
    let is_none = request.tool_choice.as_ref() == Some(&ToolChoice::Disabled);

    let tools = if is_none {
        None
    } else {
        request.tools.as_ref().map(|tools| {
            tools
                .iter()
                .map(|t| AnthropicTool {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    input_schema: {
                        let mut s = t
                            .function
                            .parameters
                            .clone()
                            .unwrap_or(serde_json::json!({"type": "object"}));
                        schema::inline_refs(&mut s);
                        s
                    },
                })
                .collect()
        })
    };

    let tool_choice = if is_none {
        None
    } else {
        request.tool_choice.as_ref().map(|tc| match tc {
            ToolChoice::Auto => serde_json::json!({"type": "auto"}),
            ToolChoice::Required => serde_json::json!({"type": "any"}),
            ToolChoice::Function { name } => serde_json::json!({"type": "tool", "name": name}),
            ToolChoice::Disabled => unreachable!(),
        })
    };

    let stop_sequences = request.stop.as_ref().map(|s| match s {
        Stop::Single(s) => vec![s.clone()],
        Stop::Multiple(v) => v.clone(),
    });

    let max_tokens = request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

    // Derive thinking config from request.extra["thinking"].
    let thinking = request.extra.get("thinking").and_then(|v| {
        if v.as_bool() == Some(true) {
            Some(ThinkingConfig {
                kind: "enabled".to_string(),
                budget_tokens: max_tokens.saturating_sub(1),
            })
        } else if let Some(obj) = v.as_object() {
            let budget = obj
                .get("budget_tokens")
                .and_then(|b| b.as_u64())
                .unwrap_or(max_tokens.saturating_sub(1) as u64) as u32;
            Some(ThinkingConfig {
                kind: "enabled".to_string(),
                budget_tokens: budget,
            })
        } else {
            None
        }
    });

    AnthropicRequest {
        model: request.model.clone(),
        messages,
        max_tokens,
        system,
        temperature: request.temperature,
        top_p: request.top_p,
        stream: request.stream,
        tools,
        tool_choice,
        stop_sequences,
        thinking,
    }
}

fn map_usage(u: &AnthropicUsage) -> Usage {
    Usage {
        prompt_tokens: u.input_tokens,
        completion_tokens: u.output_tokens,
        total_tokens: u.input_tokens + u.output_tokens,
        completion_tokens_details: None,
        prompt_cache_hit_tokens: u.cache_read_input_tokens,
        prompt_cache_miss_tokens: u.cache_creation_input_tokens,
    }
}

fn map_stop_reason(stop_reason: &Option<String>) -> Option<FinishReason> {
    stop_reason.as_ref().map(|r| match r.as_str() {
        "end_turn" => FinishReason::Stop,
        "max_tokens" => FinishReason::Length,
        "tool_use" => FinishReason::ToolCalls,
        other => FinishReason::Custom(other.to_string()),
    })
}

fn translate_response(resp: AnthropicResponse) -> ChatCompletionResponse {
    let mut content_text = String::new();
    let mut tool_calls = Vec::new();
    let mut reasoning_content = None;

    for block in &resp.content {
        match block.kind.as_str() {
            "thinking" => {
                if !block.text.is_empty() {
                    reasoning_content = Some(block.text.clone());
                }
            }
            "text" => content_text.push_str(&block.text),
            "tool_use" => {
                if let (Some(id), Some(name), Some(input)) = (&block.id, &block.name, &block.input)
                {
                    tool_calls.push(ToolCall {
                        index: None,
                        id: id.clone(),
                        kind: ToolType::Function,
                        function: FunctionCall {
                            name: name.clone(),
                            arguments: serde_json::to_string(input).unwrap_or_default(),
                        },
                    });
                }
            }
            _ => {}
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
        id: resp.id,
        object: "chat.completion".to_string(),
        created: 0,
        model: resp.model,
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content,
                tool_calls: tool_calls_opt,
                tool_call_id: None,
                name: None,
                reasoning_content,
                extra: Default::default(),
            },
            finish_reason: map_stop_reason(&resp.stop_reason),
            logprobs: None,
        }],
        usage: Some(map_usage(&resp.usage)),
        system_fingerprint: None,
    }
}

pub fn not_implemented(name: &str) -> Error {
    Error::Internal(format!("anthropic {name} not supported"))
}

// ── Auth helper ──

/// Returns true when the credential is an OAuth access token (e.g. `sk-ant-oat01-...`)
/// rather than a standard API key (`sk-ant-api03-...`).
fn is_oauth_token(credential: &str) -> bool {
    credential.contains("-oat")
}

/// Apply the correct auth header based on credential type.
///
/// OAuth tokens use `Authorization: Bearer` + the `anthropic-beta: oauth-2025-04-20`
/// feature flag. Standard API keys use the `x-api-key` header.
fn apply_auth(builder: reqwest::RequestBuilder, credential: &str) -> reqwest::RequestBuilder {
    if is_oauth_token(credential) {
        builder
            .header("Authorization", format!("Bearer {credential}"))
            .header("anthropic-beta", "oauth-2025-04-20")
    } else {
        builder.header("x-api-key", credential)
    }
}

// ── Public API ──

pub async fn chat_completion(
    client: &reqwest::Client,
    api_key: &str,
    request: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse, Error> {
    let anthropic_req = translate_request(request);
    let url = format!("{BASE_URL}/messages");

    let mut req = apply_auth(client.post(&url), api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json");
    if anthropic_req.thinking.is_some() {
        req = req.header("anthropic-beta", "interleaved-thinking-2025-05-14");
    }

    // OAuth tokens require streaming — redirect non-streaming requests to
    // the streaming path and accumulate the final response.
    if is_oauth_token(api_key) {
        let stream_resp = chat_completion_stream(client, api_key, request, &request.model).await?;
        return accumulate_stream(stream_resp, &request.model).await;
    }

    let resp = req
        .json(&anthropic_req)
        .send()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    let anthropic_resp: AnthropicResponse = resp
        .json()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    Ok(translate_response(anthropic_resp))
}

pub async fn chat_completion_stream(
    client: &reqwest::Client,
    api_key: &str,
    request: &ChatCompletionRequest,
    model: &str,
) -> Result<impl Stream<Item = Result<ChatCompletionChunk, Error>> + use<>, Error> {
    let mut anthropic_req = translate_request(request);
    anthropic_req.stream = Some(true);
    let url = format!("{BASE_URL}/messages");

    let mut req = apply_auth(client.post(&url), api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json");
    if anthropic_req.thinking.is_some() {
        req = req.header("anthropic-beta", "interleaved-thinking-2025-05-14");
    }
    let resp = req
        .json(&anthropic_req)
        .send()
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::Provider { status, body });
    }

    let model = model.to_string();
    Ok(anthropic_sse_stream(resp, model))
}

/// Accumulate a streaming response into a single ChatCompletionResponse.
/// Used for OAuth tokens which require streaming but where the caller
/// expects a non-streaming response.
async fn accumulate_stream(
    stream: impl Stream<Item = Result<ChatCompletionChunk, Error>>,
    model: &str,
) -> Result<ChatCompletionResponse, Error> {
    use futures::StreamExt;

    let mut content = String::new();
    let mut reasoning = None::<String>;
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut finish_reason = None;
    let mut usage = None;

    let mut stream = std::pin::pin!(stream);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        for choice in &chunk.choices {
            if let Some(ref c) = choice.delta.content {
                content.push_str(c);
            }
            if let Some(ref r) = choice.delta.reasoning_content {
                reasoning.get_or_insert_with(String::new).push_str(r);
            }
            if let Some(ref tcs) = choice.delta.tool_calls {
                for tc_delta in tcs {
                    let idx = tc_delta.index as usize;
                    while tool_calls.len() <= idx {
                        tool_calls.push(ToolCall {
                            index: None,
                            id: String::new(),
                            kind: ToolType::Function,
                            function: FunctionCall {
                                name: String::new(),
                                arguments: String::new(),
                            },
                        });
                    }
                    if let Some(ref id) = tc_delta.id {
                        tool_calls[idx].id.clone_from(id);
                    }
                    if let Some(ref f) = tc_delta.function {
                        if let Some(ref name) = f.name {
                            tool_calls[idx].function.name.clone_from(name);
                        }
                        if let Some(ref args) = f.arguments {
                            tool_calls[idx].function.arguments.push_str(args);
                        }
                    }
                }
            }
            if choice.finish_reason.is_some() {
                finish_reason = choice.finish_reason.clone();
            }
        }
        if chunk.usage.is_some() {
            usage = chunk.usage;
        }
    }

    let tool_calls_opt = if tool_calls.is_empty() { None } else { Some(tool_calls) };
    let message_content = if content.is_empty() && tool_calls_opt.is_some() {
        None
    } else {
        Some(serde_json::Value::String(content))
    };

    Ok(ChatCompletionResponse {
        id: String::new(),
        object: "chat.completion".to_string(),
        created: 0,
        model: model.to_string(),
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content: message_content,
                tool_calls: tool_calls_opt,
                tool_call_id: None,
                name: None,
                reasoning_content: reasoning,
                extra: Default::default(),
            },
            finish_reason,
            logprobs: None,
        }],
        usage,
        system_fingerprint: None,
    })
}

/// Streaming state: tracks chunk counter, tool call counter, cached input tokens,
/// and whether the current content block is a thinking block.
struct StreamState {
    chunk_idx: u64,
    tool_call_idx: u32,
    input_tokens: u32,
    cache_read_input_tokens: Option<u32>,
    cache_creation_input_tokens: Option<u32>,
    is_thinking_block: bool,
}

fn anthropic_sse_stream(
    resp: reqwest::Response,
    model: String,
) -> impl Stream<Item = Result<ChatCompletionChunk, Error>> {
    let byte_stream = resp.bytes_stream();
    let state = StreamState {
        chunk_idx: 0,
        tool_call_idx: 0,
        input_tokens: 0,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        is_thinking_block: false,
    };

    stream::unfold(
        (byte_stream, BytesMut::new(), model, state),
        |(mut byte_stream, mut buffer, model, mut state)| async move {
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

                    let event: SseEvent = match serde_json::from_str(data) {
                        Ok(e) => e,
                        Err(_) => {
                            buffer.advance(newline_pos + 1);
                            continue;
                        }
                    };
                    buffer.advance(newline_pos + 1);

                    match event.kind.as_str() {
                        "message_start" => {
                            if let Some(msg) = &event.message
                                && let Some(usage) = &msg.usage
                            {
                                state.input_tokens = usage.input_tokens;
                                state.cache_read_input_tokens = usage.cache_read_input_tokens;
                                state.cache_creation_input_tokens =
                                    usage.cache_creation_input_tokens;
                            }
                        }
                        "error" => {
                            let msg = if let Some(err) = &event.error {
                                format!("anthropic stream error: {}: {}", err.kind, err.message)
                            } else {
                                "anthropic stream error: unknown".to_string()
                            };
                            return Some((
                                Err(Error::Internal(msg)),
                                (byte_stream, buffer, model, state),
                            ));
                        }
                        "content_block_start" => {
                            let Some(cb) = &event.content_block else {
                                continue;
                            };
                            match cb.kind.as_str() {
                                "thinking" => state.is_thinking_block = true,
                                "tool_use" => {
                                    state.is_thinking_block = false;
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
                                                role: if state.chunk_idx == 1 {
                                                    Some(Role::Assistant)
                                                } else {
                                                    None
                                                },
                                                content: None,
                                                tool_calls: Some(vec![ToolCallDelta {
                                                    index: tool_idx,
                                                    id: cb.id.clone(),
                                                    kind: Some(ToolType::Function),
                                                    function: Some(FunctionCallDelta {
                                                        name: cb.name.clone(),
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
                                    return Some((Ok(chunk), (byte_stream, buffer, model, state)));
                                }
                                _ => state.is_thinking_block = false,
                            }
                        }
                        "content_block_stop" => {
                            state.is_thinking_block = false;
                        }
                        "content_block_delta" => {
                            let Some(delta) = &event.delta else {
                                continue;
                            };
                            match delta.kind.as_str() {
                                "thinking_delta" => {
                                    let text = delta.thinking.as_deref().unwrap_or(&delta.text);
                                    if text.is_empty() {
                                        continue;
                                    }
                                    state.chunk_idx += 1;
                                    let chunk = ChatCompletionChunk {
                                        id: format!("chatcmpl-{}", state.chunk_idx),
                                        object: "chat.completion.chunk".to_string(),
                                        created: 0,
                                        model: model.clone(),
                                        choices: vec![ChunkChoice {
                                            index: 0,
                                            delta: Delta {
                                                role: if state.chunk_idx == 1 {
                                                    Some(Role::Assistant)
                                                } else {
                                                    None
                                                },
                                                content: None,
                                                tool_calls: None,
                                                reasoning_content: Some(text.to_string()),
                                            },
                                            finish_reason: None,
                                            logprobs: None,
                                        }],
                                        usage: None,
                                        system_fingerprint: None,
                                    };
                                    return Some((Ok(chunk), (byte_stream, buffer, model, state)));
                                }
                                "text_delta" => {
                                    state.chunk_idx += 1;
                                    let chunk = ChatCompletionChunk {
                                        id: format!("chatcmpl-{}", state.chunk_idx),
                                        object: "chat.completion.chunk".to_string(),
                                        created: 0,
                                        model: model.clone(),
                                        choices: vec![ChunkChoice {
                                            index: 0,
                                            delta: Delta {
                                                role: if state.chunk_idx == 1 {
                                                    Some(Role::Assistant)
                                                } else {
                                                    None
                                                },
                                                content: Some(delta.text.clone()),
                                                tool_calls: None,
                                                reasoning_content: None,
                                            },
                                            finish_reason: None,
                                            logprobs: None,
                                        }],
                                        usage: None,
                                        system_fingerprint: None,
                                    };
                                    return Some((Ok(chunk), (byte_stream, buffer, model, state)));
                                }
                                "input_json_delta" => {
                                    let Some(partial) = &delta.partial_json else {
                                        continue;
                                    };
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
                                                        arguments: Some(partial.clone()),
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
                                    return Some((Ok(chunk), (byte_stream, buffer, model, state)));
                                }
                                _ => {}
                            }
                        }
                        "message_delta" => {
                            let Some(delta) = &event.delta else {
                                continue;
                            };
                            let finish_reason = map_stop_reason(&delta.stop_reason);
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
                                    finish_reason,
                                    logprobs: None,
                                }],
                                usage: event.usage.map(|u| Usage {
                                    prompt_tokens: state.input_tokens,
                                    completion_tokens: u.output_tokens,
                                    total_tokens: state.input_tokens + u.output_tokens,
                                    completion_tokens_details: None,
                                    prompt_cache_hit_tokens: state.cache_read_input_tokens,
                                    prompt_cache_miss_tokens: state.cache_creation_input_tokens,
                                }),
                                system_fingerprint: None,
                            };
                            return Some((Ok(chunk), (byte_stream, buffer, model, state)));
                        }
                        "message_stop" => return None,
                        _ => {}
                    }
                    continue;
                }

                match byte_stream.try_next().await {
                    Ok(Some(bytes)) => {
                        buffer.extend_from_slice(&bytes);
                    }
                    Ok(None) => return None,
                    Err(e) => {
                        return Some((
                            Err(Error::Internal(format!("stream error: {e}"))),
                            (byte_stream, buffer, model, state),
                        ));
                    }
                }
            }
        },
    )
}
