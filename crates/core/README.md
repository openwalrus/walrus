# cydonia-core

Core abstractions for the Unified LLM Interface.

## Overview

This crate provides the foundational types and traits for building LLM applications:

- `LLM` - Provider trait for LLM backends
- `Agent` - Trait for building tool-using agents
- `Chat` - Chat session management with streaming support
- `Message` - Chat message types (user, assistant, system, tool)
- `Tool` / `ToolCall` - Function calling abstractions
- `StreamChunk` - Streaming response handling

## License

MIT
