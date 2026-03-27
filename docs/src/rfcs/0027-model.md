# 0027 - Model

- Feature Name: Model Abstraction Layer
- Start Date: 2026-01-25
- Discussion: [#27](https://github.com/crabtalk/crabtalk/issues/27)
- Crates: model, core

## Summary

A provider registry that wraps multiple LLM backends (OpenAI, Anthropic, Google,
Bedrock, Azure) behind a unified `Model` trait, with per-model provider
instances, runtime model switching, and retry logic with exponential backoff.

## Motivation

The daemon talks to LLMs. Which LLM, from which provider, through which API —
that's configuration, not architecture. The agent code should call `model.send()`
and not care whether it's hitting Anthropic directly or an OpenAI-compatible
proxy.

This requires:

- A single trait that all providers implement.
- A registry that maps model names to provider instances.
- Runtime switching between models without restarting.
- Retry logic for transient failures (rate limits, timeouts).
- Type conversion between crabtalk's message types and each provider's wire
  format.

## Design

### Model trait (core)

Defined in `wcore::model`:

```rust
pub trait Model: Clone + Send + Sync {
    async fn send(&self, request: &Request) -> Result<Response>;
    fn stream(&self, request: Request) -> impl Stream<Item = Result<StreamChunk>>;
    fn context_limit(&self, model: &str) -> usize;
    fn active_model(&self) -> String;
}
```

The trait is in core because agents are generic over `Model`. The implementation
lives in the model crate.

### Provider

Wraps `crabllm_provider::Provider` (the external multi-backend LLM library)
behind the `Model` trait. Each `Provider` instance is bound to a specific model
name and carries:

- The backend connection (OpenAI, Anthropic, Google, Bedrock, Azure).
- A shared HTTP client.
- Retry config: `max_retries` (default 2) and `timeout` (default 30s).

Base URL normalization strips endpoint suffixes (`/chat/completions`,
`/messages`) so both bare origins and full paths work in config.

### ProviderRegistry

Implements `Model` by routing requests to the correct provider based on the
model name in the request.

```
ProviderRegistry
├── providers: BTreeMap<String, Provider>   # keyed by model name
├── active: String                          # default model
└── client: reqwest::Client                 # shared across providers
```

- **Construction**: one `ProviderDef` can list multiple model names. Each gets
  its own `Provider` instance. Duplicate model names across definitions are
  rejected at validation time.
- **Routing**: `send()` and `stream()` look up the provider by `request.model`.
  Callers get a clone of the provider — the registry lock is not held during
  LLM calls.
- **Switching**: `switch(model)` changes the active default. Agents can still
  override per-request via the model field.
- **Hot add/remove**: providers can be added or removed at runtime without
  rebuilding the registry.

### Retry logic

Non-streaming `send()` retries transient errors (rate limits, timeouts) with
exponential backoff and full jitter:

- Initial backoff: 100ms, doubling each retry.
- Jitter: random duration in `[backoff/2, backoff]`.
- Max retries: configurable per provider (default 2).
- Non-transient errors (auth failures, invalid requests) fail immediately.

Streaming does not retry — the connection is already established.

### Type conversion

A `convert` module translates between `wcore::model` types (Request, Response,
Message, StreamChunk) and `crabllm_core` types (ChatCompletionRequest,
ChatCompletionResponse). This isolates the external library's types from the
rest of the codebase.

## Alternatives

**Direct provider calls without a registry.** Each agent holds its own provider.
Rejected because runtime model switching and centralized configuration require
a shared registry.

**Trait objects instead of enum dispatch.** `Box<dyn Model>` instead of the
concrete `Provider` enum. Rejected because `Model` has generic return types
(impl Stream) that prevent object safety. The enum dispatch via
`crabllm_provider::Provider` handles this naturally.

## Unresolved Questions

- Should the registry support fallback chains (try provider A, fall back to B)?
- Should streaming requests retry on connection failures before the first chunk?
