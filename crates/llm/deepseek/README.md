# walrus-deepseek

DeepSeek LLM provider for walrus.

## Overview

Implements the `LLM` trait for DeepSeek API, supporting:

- `deepseek-chat` - Standard chat model
- `deepseek-reasoner` - Reasoning model with thinking mode
- Streaming responses
- Tool calling with thinking mode

## Usage

```rust
use walrus::{DeepSeek, LLM};

let provider = DeepSeek::new(client, "your-api-key")?;
```

## License

MIT
