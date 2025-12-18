//! Tests for the response module

use cydonia_core::{Response, StreamChunk};

const DEEPSEEK_RESPONSE_JSON: &str = include_str!("../templates/deepseek/response.json");
const DEEPSEEK_STREAM_CHUNK_JSON: &str = include_str!("../templates/deepseek/stream.json");

#[test]
fn parse_response() {
    let _response: Response = serde_json::from_str(DEEPSEEK_RESPONSE_JSON).unwrap();
}

#[test]
fn parse_stream_chunk() {
    let _stream_chunks: Vec<StreamChunk> =
        serde_json::from_str(DEEPSEEK_STREAM_CHUNK_JSON).unwrap();
}
