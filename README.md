# Cydonia

Unified LLM Interface - A Rust framework for building LLM-powered agents.

## Features

- Unified interface for multiple LLM providers
- Streaming support with thinking mode
- Tool calling with automatic argument accumulation
- Agent framework for building autonomous assistants

## Crates

| Crate | Description |
|-------|-------------|
| `cydonia` | Umbrella crate re-exporting all components |
| `cydonia-core` | Core abstractions (LLM, Agent, Chat, Message) |
| `cydonia-deepseek` | DeepSeek provider implementation |
| `cydonia-cli` | Command line interface |

## Quick Start

```rust
use cydonia::{Chat, DeepSeek, LLM, Message};

let client = reqwest::Client::new();
let provider = DeepSeek::new(client, "your-api-key")?;
let mut chat = provider.chat(config);

let response = chat.send(Message::user("Hello!")).await?;
```

## License

MIT
