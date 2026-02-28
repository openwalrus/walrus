//! Default context limits for known model families.
//!
//! Provides a static map from model ID prefixes to context window sizes.
//! Used by the model registry for per-model context limit resolution (DD#68).

/// Returns the default context limit (in tokens) for a known model ID.
///
/// Uses prefix matching against known model families. Unknown models
/// return 8192 as a conservative default.
pub fn default_context_limit(model_id: &str) -> usize {
    // Claude family
    if model_id.starts_with("claude-") {
        return 200_000;
    }
    // GPT-4o / GPT-4-turbo family
    if model_id.starts_with("gpt-4o") || model_id.starts_with("gpt-4-turbo") {
        return 128_000;
    }
    // GPT-4 (non-turbo)
    if model_id.starts_with("gpt-4") {
        return 8_192;
    }
    // GPT-3.5
    if model_id.starts_with("gpt-3.5") {
        return 16_385;
    }
    // OpenAI o-series (o1, o3, o4)
    if model_id.starts_with("o1") || model_id.starts_with("o3") || model_id.starts_with("o4") {
        return 200_000;
    }
    // Grok family
    if model_id.starts_with("grok-") {
        return 131_072;
    }
    // DeepSeek family
    if model_id.starts_with("deepseek-") {
        return 64_000;
    }
    // Qwen family
    if model_id.starts_with("qwen-") || model_id.starts_with("qwq-") {
        return 32_768;
    }
    // Kimi / Moonshot family
    if model_id.starts_with("kimi-") || model_id.starts_with("moonshot-") {
        return 128_000;
    }
    // Unknown model — conservative default
    8_192
}
