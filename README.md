# Crabtalk

[![Crates.io][crates-badge]][crates]
[![Docs][docs-badge]][docs]
[![Discord][discord-badge]][discord]

**The composable agent runtime.** Compact daemon core. Memory, channels,
tools — all hooks. Use what you need, skip what you don't.

```bash
curl -fsSL https://crabtalk.ai/install.sh | sh
```

Or `cargo install crabtalk`. See the [installation guide][install] for details.

## Quick Start

```bash
# Start the daemon
crabtalk daemon install

# Chat with your agent
crabtalk attach
```

Full config reference: [configuration][config].

## How It Works

Crabtalk is a daemon that runs [agents] and dispatches tools. The daemon
ships with built-in [tools] (shell, task delegation, memory),
[MCP][mcp] server integration, and [skills] (Markdown prompt files).

Heavier capabilities live outside the daemon as [extensions][services] —
managed child processes you add or remove in config:

| Service            | What it does                                 |
| ------------------ | -------------------------------------------- |
| [Search][search]   | Meta-search aggregator                       |
| [Gateway][gateway] | Telegram adapter                             |

The daemon stays small. Services scale independently.

## Learn More

- [Quickstart][quickstart] — first agent in 2 minutes
- [Configuration][config] — crab.toml reference
- [Providers][providers] — connect any LLM
- [Extensions][services] — how extensions work
- [Architecture][runtime] — runtime, event loop, hooks
- [Why we built Crabtalk][blog]

## License

GPL-3.0

<!-- badges -->

[crates-badge]: https://img.shields.io/crates/v/crabtalk.svg
[crates]: https://crates.io/crates/crabtalk
[docs-badge]: https://img.shields.io/badge/docs-crabtalk.ai-blue
[docs]: https://crabtalk.ai/docs/crabtalk
[discord-badge]: https://img.shields.io/discord/1481168707391852659?label=discord
[discord]: https://discord.gg/XxyxfNX3Fn

<!-- docs -->

[install]: https://crabtalk.ai/docs/crabtalk/getting-started/installation
[quickstart]: https://crabtalk.ai/docs/crabtalk/getting-started/quickstart
[config]: https://crabtalk.ai/docs/crabtalk/getting-started/configuration
[providers]: https://crabtalk.ai/docs/crabtalk/models/providers
[remote]: https://crabtalk.ai/docs/crabtalk/models/remote
[agents]: https://crabtalk.ai/docs/development/concepts/agents
[runtime]: https://crabtalk.ai/docs/development/concepts/runtime
[services]: https://crabtalk.ai/docs/crabtalk/extensions
[search]: https://crabtalk.ai/docs/crabtalk/extensions/search
[gateway]: https://crabtalk.ai/docs/crabtalk/extensions/gateway
[tools]: https://crabtalk.ai/docs/development/tools/built-in
[mcp]: https://crabtalk.ai/docs/development/tools/mcp
[skills]: https://crabtalk.ai/docs/development/tools/skills
[blog]: https://crabtalk.ai/blog/why-we-built-crabtalk
