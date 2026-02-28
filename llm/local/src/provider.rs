//! LLM trait implementation for the Local provider.

use crate::Local;
use anyhow::Result;
use async_stream::try_stream;
use compact_str::CompactString;
use futures_core::Stream;
use llm::{
    Choice, CompletionMeta, Delta, FunctionCall, General, LLM, Message, Response, Role,
    StreamChunk, ToolCall, Usage,
};
use std::collections::HashMap;

impl LLM for Local {
    type ChatConfig = General;

    async fn send(&self, config: &General, messages: &[Message]) -> Result<Response> {
        let request = build_request(config, messages);
        let resp = self.model.send_chat_request(request).await?;
        Ok(to_response(resp))
    }

    fn stream(
        &self,
        config: General,
        messages: &[Message],
        _usage: bool,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send {
        let model = self.model.clone();
        let messages = messages.to_vec();
        try_stream! {
            let request = build_request(&config, &messages);
            let mut stream = model.stream_chat_request(request).await?;
            while let Some(resp) = stream.next().await {
                match resp {
                    mistralrs::Response::Chunk(chunk) => {
                        yield to_stream_chunk(chunk);
                    }
                    mistralrs::Response::Done(_) => break,
                    mistralrs::Response::InternalError(e)
                    | mistralrs::Response::ValidationError(e) => {
                        Err(anyhow::anyhow!("{e}"))?;
                    }
                    mistralrs::Response::ModelError(msg, _) => {
                        Err(anyhow::anyhow!("model error: {msg}"))?;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Build a mistralrs `RequestBuilder` from walrus `General` config and messages.
fn build_request(config: &General, messages: &[Message]) -> mistralrs::RequestBuilder {
    let mut builder = mistralrs::RequestBuilder::new();

    for msg in messages {
        match msg.role {
            Role::System => {
                builder =
                    builder.add_message(mistralrs::TextMessageRole::System, &msg.content);
            }
            Role::User => {
                builder =
                    builder.add_message(mistralrs::TextMessageRole::User, &msg.content);
            }
            Role::Assistant => {
                if msg.tool_calls.is_empty() {
                    builder = builder
                        .add_message(mistralrs::TextMessageRole::Assistant, &msg.content);
                } else {
                    let tool_calls = msg
                        .tool_calls
                        .iter()
                        .map(|tc| mistralrs::ToolCallResponse {
                            id: tc.id.to_string(),
                            tp: mistralrs::ToolCallType::Function,
                            function: mistralrs::CalledFunction {
                                name: tc.function.name.to_string(),
                                arguments: tc.function.arguments.clone(),
                            },
                            index: tc.index as usize,
                        })
                        .collect();
                    builder = builder.add_message_with_tool_call(
                        mistralrs::TextMessageRole::Assistant,
                        &msg.content,
                        tool_calls,
                    );
                }
            }
            Role::Tool => {
                builder = builder.add_tool_message(&msg.content, &msg.tool_call_id);
            }
        }
    }

    if let Some(tools) = &config.tools {
        let mr_tools = tools
            .iter()
            .map(|t| {
                let params: HashMap<String, serde_json::Value> =
                    serde_json::from_value(
                        serde_json::to_value(&t.parameters).unwrap_or_default(),
                    )
                    .unwrap_or_default();
                mistralrs::Tool {
                    tp: mistralrs::ToolType::Function,
                    function: mistralrs::Function {
                        description: Some(t.description.clone()),
                        name: t.name.to_string(),
                        parameters: Some(params),
                    },
                }
            })
            .collect();
        builder = builder.set_tools(mr_tools);
    }

    if let Some(tool_choice) = &config.tool_choice {
        let mr_choice = match tool_choice {
            llm::ToolChoice::None => mistralrs::ToolChoice::None,
            llm::ToolChoice::Auto | llm::ToolChoice::Required => mistralrs::ToolChoice::Auto,
            llm::ToolChoice::Function(name) => {
                mistralrs::ToolChoice::Tool(mistralrs::Tool {
                    tp: mistralrs::ToolType::Function,
                    function: mistralrs::Function {
                        description: None,
                        name: name.to_string(),
                        parameters: None,
                    },
                })
            }
        };
        builder = builder.set_tool_choice(mr_choice);
    }

    builder
}

/// Convert a mistralrs `ChatCompletionResponse` to a walrus `Response`.
fn to_response(resp: mistralrs::ChatCompletionResponse) -> Response {
    let choices = resp
        .choices
        .into_iter()
        .map(|c| Choice {
            index: c.index as u32,
            delta: Delta {
                role: Some(Role::Assistant),
                content: c.message.content,
                reasoning_content: c.message.reasoning_content,
                tool_calls: c
                    .message
                    .tool_calls
                    .map(|tcs| tcs.into_iter().map(convert_tool_call).collect()),
            },
            finish_reason: parse_finish_reason(&c.finish_reason),
            logprobs: None,
        })
        .collect();

    Response {
        meta: CompletionMeta {
            id: CompactString::from(&resp.id),
            object: CompactString::from(&resp.object),
            created: resp.created,
            model: CompactString::from(&resp.model),
            system_fingerprint: Some(CompactString::from(&resp.system_fingerprint)),
        },
        choices,
        usage: convert_usage(&resp.usage),
    }
}

/// Convert a mistralrs `ChatCompletionChunkResponse` to a walrus `StreamChunk`.
fn to_stream_chunk(chunk: mistralrs::ChatCompletionChunkResponse) -> StreamChunk {
    let choices = chunk
        .choices
        .into_iter()
        .map(|c| Choice {
            index: c.index as u32,
            delta: Delta {
                role: Some(Role::Assistant),
                content: c.delta.content,
                reasoning_content: c.delta.reasoning_content,
                tool_calls: c
                    .delta
                    .tool_calls
                    .map(|tcs| tcs.into_iter().map(convert_tool_call).collect()),
            },
            finish_reason: c.finish_reason.as_ref().and_then(|r| parse_finish_reason(r)),
            logprobs: None,
        })
        .collect();

    StreamChunk {
        meta: CompletionMeta {
            id: CompactString::from(&chunk.id),
            object: CompactString::from(&chunk.object),
            created: chunk.created as u64,
            model: CompactString::from(&chunk.model),
            system_fingerprint: Some(CompactString::from(&chunk.system_fingerprint)),
        },
        choices,
        usage: chunk.usage.as_ref().map(convert_usage),
    }
}

/// Convert a mistralrs `ToolCallResponse` to a walrus `ToolCall`.
fn convert_tool_call(tc: mistralrs::ToolCallResponse) -> ToolCall {
    ToolCall {
        id: CompactString::from(&tc.id),
        index: tc.index as u32,
        call_type: CompactString::from("function"),
        function: FunctionCall {
            name: CompactString::from(&tc.function.name),
            arguments: tc.function.arguments,
        },
    }
}

/// Convert a mistralrs `Usage` to a walrus `Usage`.
fn convert_usage(u: &mistralrs::Usage) -> Usage {
    Usage {
        prompt_tokens: u.prompt_tokens as u32,
        completion_tokens: u.completion_tokens as u32,
        total_tokens: u.total_tokens as u32,
        prompt_cache_hit_tokens: None,
        prompt_cache_miss_tokens: None,
        completion_tokens_details: None,
    }
}

/// Parse a finish reason string into a walrus `FinishReason`.
fn parse_finish_reason(reason: &str) -> Option<llm::FinishReason> {
    match reason {
        "stop" => Some(llm::FinishReason::Stop),
        "length" => Some(llm::FinishReason::Length),
        "content_filter" => Some(llm::FinishReason::ContentFilter),
        "tool_calls" => Some(llm::FinishReason::ToolCalls),
        _ => None,
    }
}
