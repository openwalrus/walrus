# Walrus

[![Crates.io](https://img.shields.io/crates/v/openwalrus.svg)](https://crates.io/crates/openwalrus)

**Run autonomous agents with built-in LLM inference. No API keys. No cloud.
Just one binary.**

```bash
curl -fsSL https://openwalrus.xyz/install.sh | sh
```

Or install with Cargo:

```bash
cargo install openwalrus
```

## What It Does

- **Local inference** — runs LLMs on your machine (Metal on macOS, CUDA on Linux)
- **Persistent memory** — agents remember across sessions (SQLite + FTS5)
- **Built-in tools** — file I/O, shell, MCP servers, cron scheduling
- **Multi-channel** — talk to your agents from the terminal, Telegram, or Discord
- **Skills** — extend agents with Markdown prompt files, no code needed

## Quick Start

```bash
# 1. Start the daemon
walrus daemon

# 2. Chat with your agent
walrus attach
```

Models are configured in `~/.openwalrus/config.toml`. Point it at a local model
or any OpenAI-compatible API:

```toml
[model]
model = "qwen3:4b"
```

## Cloud Providers

walrus works with OpenAI, Anthropic, DeepSeek, and other OpenAI-compatible
APIs. Local inference is the default — cloud is opt-in.

## License

GPL-3.0
