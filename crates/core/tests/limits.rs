//! Tests for the default context limit map.

use walrus_core::model::default_context_limit;

#[test]
fn limit_deepseek() {
    assert_eq!(default_context_limit("deepseek-chat"), 64_000);
    assert_eq!(default_context_limit("deepseek-reasoner"), 64_000);
}

#[test]
fn limit_gpt4o() {
    assert_eq!(default_context_limit("gpt-4o"), 128_000);
    assert_eq!(default_context_limit("gpt-4o-mini"), 128_000);
}

#[test]
fn limit_gpt4_turbo() {
    assert_eq!(default_context_limit("gpt-4-turbo"), 128_000);
}

#[test]
fn limit_gpt4_base() {
    assert_eq!(default_context_limit("gpt-4"), 8_192);
}

#[test]
fn limit_gpt35() {
    assert_eq!(default_context_limit("gpt-3.5-turbo"), 16_385);
}

#[test]
fn limit_claude() {
    assert_eq!(default_context_limit("claude-3-sonnet"), 200_000);
    assert_eq!(default_context_limit("claude-4-opus"), 200_000);
}

#[test]
fn limit_grok() {
    assert_eq!(default_context_limit("grok-3"), 131_072);
}

#[test]
fn limit_qwen() {
    assert_eq!(default_context_limit("qwen-plus"), 32_768);
    assert_eq!(default_context_limit("qwq-32b"), 32_768);
}

#[test]
fn limit_o_series() {
    assert_eq!(default_context_limit("o1-preview"), 200_000);
    assert_eq!(default_context_limit("o3-mini"), 200_000);
}

#[test]
fn limit_unknown() {
    assert_eq!(default_context_limit("foobar-model"), 8_192);
}

#[test]
fn limit_local_unknown() {
    assert_eq!(default_context_limit("microsoft/Phi-3"), 8_192);
}
