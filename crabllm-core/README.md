# crabllm-core

[![crates.io][badge]][crate]

Core types for the [crabllm](https://github.com/clearloop/crabllm) LLM API gateway.

- **Config** — `GatewayConfig`, `ProviderConfig`, `ProviderKind`, `KeyConfig`, `StorageConfig`, `PricingConfig`
- **Types** — OpenAI-compatible wire format: `ChatCompletionRequest`/`Response`/`Chunk`, `EmbeddingRequest`/`Response`, `ImageRequest`, `AudioSpeechRequest`, `ModelList`
- **Error** — `Error` enum with transient detection for retry logic, `ApiError` for OpenAI-format error responses
- **Storage** — async KV `Storage` trait with `MemoryStorage`, `SqliteStorage` (`storage-sqlite`), `RedisStorage` (`storage-redis`)
- **Extension** — `Extension` trait with request pipeline hooks

## License

MIT OR Apache-2.0

[badge]: https://img.shields.io/crates/v/crabllm-core.svg
[crate]: https://crates.io/crates/crabllm-core
