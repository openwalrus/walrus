//! LLM trait implementation for the Claude (Anthropic) provider.

use super::{Claude, Request, stream::Event};
use anyhow::Result;
use async_stream::try_stream;
use compact_str::CompactString;
use futures_core::Stream;
use futures_util::StreamExt;
use reqwest::Method;
use wcore::model::{
    Choice, CompletionMeta, CompletionTokensDetails, Delta, FinishReason, LLM, Message, Response,
    StreamChunk, Usage,
};

/// Raw Anthropic non-streaming response.
#[derive(serde::Deserialize)]
struct AnthropicResponse {
    id: CompactString,
    model: CompactString,
    content: Vec<ContentBlock>,
    stop_reason: Option<CompactString>,
    usage: AnthropicUsage,
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: CompactString,
        name: CompactString,
        input: serde_json::Value,
    },
}

#[derive(serde::Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

impl LLM for Claude {
    type ChatConfig = Request;

    async fn send(&self, req: &Request, messages: &[Message]) -> Result<Response> {
        let body = req.messages(messages);
        tracing::trace!("request: {}", serde_json::to_string(&body)?);
        let text = self
            .client
            .request(Method::POST, &self.endpoint)
            .headers(self.headers.clone())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        tracing::trace!("response: {text}");
        let raw: AnthropicResponse = serde_json::from_str(&text)?;
        Ok(to_response(raw))
    }

    fn stream(
        &self,
        req: Request,
        messages: &[Message],
        _usage: bool,
    ) -> impl Stream<Item = Result<StreamChunk>> {
        let body = req.messages(messages).stream();
        if let Ok(body) = serde_json::to_string(&body) {
            tracing::trace!("request: {}", body);
        }
        let request = self
            .client
            .request(Method::POST, &self.endpoint)
            .headers(self.headers.clone())
            .json(&body);

        try_stream! {
            let response = request.send().await?;
            let mut stream = response.bytes_stream();
            let mut buf = String::new();
            while let Some(Ok(bytes)) = stream.next().await {
                buf.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(pos) = buf.find("\n\n") {
                    let block = buf[..pos].to_owned();
                    buf = buf[pos + 2..].to_owned();
                    if let Some(chunk) = parse_sse_block(&block) {
                        yield chunk;
                    }
                }
            }
            // Handle any remaining data in buffer.
            if !buf.trim().is_empty()
                && let Some(chunk) = parse_sse_block(&buf) {
                yield chunk;
            }
        }
    }
}

/// Parse a single SSE block (may contain `event:` and `data:` lines).
fn parse_sse_block(block: &str) -> Option<StreamChunk> {
    let mut data_str = None;
    for line in block.lines() {
        if let Some(d) = line.strip_prefix("data: ") {
            data_str = Some(d.trim());
        }
    }
    let data = data_str?;
    if data == "[DONE]" {
        return None;
    }
    match serde_json::from_str::<Event>(data) {
        Ok(event) => event.into_chunk(),
        Err(e) => {
            tracing::warn!("failed to parse anthropic event: {e}, data: {data}");
            None
        }
    }
}

/// Convert an Anthropic response to the unified `Response` format.
fn to_response(raw: AnthropicResponse) -> Response {
    let mut content = String::new();
    let mut tool_calls = Vec::new();

    for block in raw.content {
        match block {
            ContentBlock::Text { text } => {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(&text);
            }
            ContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(wcore::model::ToolCall {
                    id,
                    index: tool_calls.len() as u32,
                    call_type: "function".into(),
                    function: wcore::model::FunctionCall {
                        name,
                        arguments: serde_json::to_string(&input).unwrap_or_default(),
                    },
                });
            }
        }
    }

    let finish_reason = raw.stop_reason.as_deref().map(|r| match r {
        "end_turn" | "stop" => FinishReason::Stop,
        "max_tokens" => FinishReason::Length,
        "tool_use" => FinishReason::ToolCalls,
        _ => FinishReason::Stop,
    });

    Response {
        meta: CompletionMeta {
            id: raw.id,
            object: "chat.completion".into(),
            model: raw.model,
            ..Default::default()
        },
        choices: vec![Choice {
            index: 0,
            delta: Delta {
                role: Some(wcore::model::Role::Assistant),
                content: Some(content),
                reasoning_content: None,
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                },
            },
            finish_reason,
            logprobs: None,
        }],
        usage: Usage {
            prompt_tokens: raw.usage.input_tokens,
            completion_tokens: raw.usage.output_tokens,
            total_tokens: raw.usage.input_tokens + raw.usage.output_tokens,
            prompt_cache_hit_tokens: None,
            prompt_cache_miss_tokens: None,
            completion_tokens_details: Some(CompletionTokensDetails {
                reasoning_tokens: None,
            }),
        },
    }
}
