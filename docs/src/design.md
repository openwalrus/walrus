# Walrus Platform Architecture

This document describes the architecture of the Walrus workspace — what exists today
and the principles guiding its evolution into a full agent platform.

## Design Principles

**General-purpose libraries, platform-specific composition.** The relationship between the
library crates and the Gateway is analogous to tower and axum. Library crates solve specific
problems without any opinion about how they are composed. The Gateway is the opinionated
layer that wires them into the Walrus platform.

**walrus-core as the shared vocabulary.** All fundamental traits and types live in a single
core crate with minimal dependencies — no database drivers, no platform SDKs. Concrete
implementations live in dedicated crates.

**Runtime as the intelligence layer.** The Runtime composes LLM providers, memory, skills,
and MCP tools into a unified agent execution engine. The Runtime is the brain; the Gateway
is the face.

**Trait-first extensibility.** Each subsystem introduces one primary trait. Concrete
implementations are just one side of that trait; anyone can bring their own.

**No over-abstraction.** Each new trait has the smallest surface that solves the problem.

---

## Workspace Layout

### Current Crates

| Crate | Path | Depends On | Purpose |
|-------|------|------------|---------|
| walrus-llm | crates/llm | — | LLM trait, Message, Tool, Config, StreamChunk |
| walrus-core | crates/core | walrus-llm | Agent, Chat, Memory, InMemory |
| walrus-deepseek | crates/llm/deepseek | walrus-llm | DeepSeek LLM provider |
| walrus-runtime | crates/runtime | walrus-core, walrus-llm, walrus-deepseek | Runtime, Provider, Handler, team composition |

### Planned Crates

| Crate | Path | Purpose | Phase |
|-------|------|---------|-------|
| walrus-sqlite | crates/sqlite | SqliteMemory via SQLite + FTS5 | 1 |
| walrus-telegram | crates/telegram | Telegram channel adapter | 2 |
| walrus-gateway | crates/gateway | WebSocket server, sessions, auth, crons | 3 |

```text
  platform    ┃            walrus-gateway
  ━━━━━━━━━━━━╋━━━━━━━━/━━━━━━|━━━━━━\━━━━━━━━
  runtime     ┃   walrus-runtime     |  sqlite
              ┃  / |    |     \  telegram  |
  core        ┃ llm | deepseek rmcp  |    |
              ┃   core              core core
              ┃    |
              ┃   llm
```

---

## walrus-llm

Unified LLM interface. Defines the `LLM` trait with `send` (request/response) and
`stream` (SSE chunks) methods. Provider-agnostic types: Message, Role, Response,
StreamChunk, Tool, ToolCall, ToolChoice, Config, General, Usage, FinishReason.
Includes `estimate_tokens` for rough token counting.

---

## walrus-core

The shared vocabulary of the workspace. Depends only on walrus-llm.

**Agent** — Pure config struct: name, description, system_prompt, tools (`Vec<String>`).
Builder-style API with `new()`, `system_prompt()`, `description()`, `tool()`.

**Chat** — Session state: agent_name + message history (`Vec<Message>`).

**Memory trait** — Structured knowledge (not chat history). Key-value store with
`get`, `set`, `remove`, `entries`, and `compile` (formats entries as XML for system
prompt injection). Bounds: `Clone + Send + Sync`. InMemory provides a `Vec`-backed
default implementation.

**with_memory** — Helper that appends `memory.compile()` to an agent's system prompt.

---

## walrus-runtime

The central composition point. Composes LLM providers, agents, tools, and chat sessions
into a unified execution engine.

**Provider** — Static dispatch enum over LLM implementations (currently DeepSeek).
Delegates `send`, `stream`, and `context_limit` to the underlying provider.

**Tool dispatch** — BTreeMap-based tool registry. `register()` accepts a Tool schema
and a type-erased async handler. `dispatch()` calls handlers for each ToolCall and
collects results. The `send()` and `stream()` methods loop tool dispatch up to 16
rounds automatically.

**Compaction** — Per-agent compaction functions that trim message history.
`needs_compaction()` checks if token usage exceeds 80% of the context limit.

**Team composition** — `build_team()` registers worker agents as tools on a leader.
`worker_tool()` creates a standard `{ input: string }` tool schema. `extract_input()`
parses the input field from tool call arguments.

---

## Platform Vision

Walrus is evolving from an agent library into a full agent platform. The extension
adds persistent memory (async Memory trait + SQLite/FTS5), messaging channels,
Markdown-based skills, MCP tool integration, and a WebSocket gateway with sessions,
authentication, and cron scheduling.

Memory search uses hybrid BM25 + vector retrieval with weighted merge, MMR
re-ranking for diversity, and temporal decay for recency. Before context
compaction, a silent memory flush turn persists important facts. Sessions are
scoped by trust level (main, DM, group, cron) with tool access controlled
per-scope.

Detailed plans for each phase live in the [plan/](./plan/) directory.

- **[Phase 1: Core](./plan/phase1-core.md)** — Expand walrus-core traits and types,
  create walrus-sqlite.
- **[Phase 2: Runtime](./plan/phase2-runtime.md)** — Integrate memory/skills/MCP into
  Runtime, implement Telegram channel adapter.
- **[Phase 3: Gateway](./plan/phase3-gateway.md)** — WebSocket server, sessions, auth,
  channel routing, cron scheduler.

---

## Design Decisions

1. **Embedder placement.** Keep in walrus-core. walrus-sqlite depends on walrus-core
   (not walrus-runtime), and SqliteMemory needs an optional Embedder for vector search.

2. **Channel adapter dependencies.** Use reqwest directly (already in workspace deps).
   Avoids adding teloxide as a heavy dependency.

3. **Channel-to-agent routing.** Separate routing table in the Gateway. Decouples agent
   configuration from channel configuration. Rules evaluated in order: match by
   (platform, channel_id), then (platform) catch-all, then default agent fallback.

4. **Skill trigger strategy.** Keyword matching first. Simple, no ML dependency.
   Upgrade to embedding similarity later if needed.

5. **Memory flush strategy.** Tool-based ("remember" command via Memory::store).
   Simpler than LLM-summarized, gives the user explicit control.

6. **Cron persistence and isolation.** Config-only for v1 (no SQLite persistence).
   Each cron job runs in an isolated session with a fresh ID per run. Cron queue
   separate from inbound message processing.

7. **Memory flush before compaction.** Before trimming context, Runtime triggers
   a silent LLM turn prompting the model to persist important facts via
   Memory::store. Prevents knowledge loss during long sessions.

8. **Session scoping.** Sessions keyed by (agent_id, scope). Scopes: Main
   (full trust), Dm (per-peer), Group (per-group), Cron (per-job, fresh each
   run). Scope determines trust level and tool access.

9. **Tool access control.** Agent's `tools` field is the allowlist. Gateway
   adds a deny layer for untrusted session scopes (Dm, Group). No multi-layer
   policy system in v1.

10. **DM safety.** Gateway-level concern. Unknown DM senders ignored by default
    until approved. Configurable per-channel.
