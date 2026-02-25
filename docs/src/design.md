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

### Performance

**Static dispatch by default.** No `dyn Trait` on hot paths. Use RPITIT for async
trait methods, generics for composition, and enum dispatch when multiple concrete
types are needed. Dynamic dispatch is reserved for user-provided plugin points only.

**Zero-copy where data flows.** Use `Bytes`/`BytesMut` for WebSocket frames and
LLM streaming. Use `Cow<'_, str>` in APIs that receive both borrowed and owned
strings. Use serde `#[serde(borrow)]` for zero-copy deserialization of LLM
responses when the input buffer outlives the parsed struct.

**Allocation-conscious data types.** Use `SmallVec` for collections that are almost
always small (tool arguments, tags, attachments). Use `CompactString` for short
identity strings (tool names, agent names, model IDs). Pre-allocate with
`with_capacity` when size is known. Reuse buffers in hot loops via `clear()`.

**Flat, cache-friendly structures.** Prefer `Vec<T>` with index references over
pointer-heavy trees. Keep message history as a flat vec. Avoid deep `Box<Node>`
chains.

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
| walrus-protocol | app/protocol | Walrus Protocol wire types (ClientMessage, ServerMessage) | 3 |
| walrus-gateway | app/gateway | WebSocket server, sessions, auth, crons | 3 |
| walrus-client | app/client | Client library (WebSocket connection to gateway) | 4 |
| walrus-cli | app/cli | Independent CLI application (direct + gateway modes) | 4 |

```text
  app/        ┃  walrus-cli ──→ walrus-client
              ┃    |  (gateway)     |
              ┃    |           walrus-protocol
              ┃    |  (direct)      |
              ┃    ↓           walrus-gateway
  ━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━|━━━━━━━━━
  runtime     ┃  walrus-runtime    |  sqlite
              ┃  / |    |    \  telegram  |
  core        ┃ llm | deepseek rmcp |    |
              ┃   core             core core
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
  channel routing, cron scheduler, protocol types.
- **[Phase 4: CLI & Client](./plan/phase4-cli.md)** — Client library, CLI with
  direct mode and gateway mode, interactive chat REPL.

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

7. **Memory flush before compaction.** Runtime orchestrates: silent LLM turn
   via Provider, then Memory::store, then Compactor trims. Compactor stays
   sync and stateless.

8. **Session scoping.** Gateway owns Session (agent_id, scope, trust level,
   session ID). Core's Chat stays minimal (agent_name + messages). Scopes:
   Main (full trust), Dm (per-peer), Group (per-group), Cron (per-job, fresh
   each run).

9. **Tool access control.** Agent's `tools` field is the allowlist. Gateway
   adds a deny layer for untrusted session scopes (Dm, Group). No multi-layer
   policy system in v1.

10. **DM safety.** Gateway-level concern. Unknown DM senders ignored by default
    until approved. Configurable per-channel.

11. **No dyn dispatch for core traits.** Memory and Embedder use RPITIT async
    methods with generics, following the LLM/Provider pattern. Runtime is
    generic over Memory. SqliteMemory is generic over Embedder. Enum dispatch
    (like Provider) used when multiple concrete types needed.

12. **Skill config format.** TOML frontmatter (+++delimited), not YAML.

13. **Identity strings use `CompactString`.** Agent names, tool names, model IDs,
    role labels — short strings that are created once and compared often. Up to
    24 bytes inline, no heap allocation. Add `compact_str` to workspace deps.

14. **`Bytes` for I/O buffers.** WebSocket frames and LLM streaming responses use
    `bytes::Bytes`/`BytesMut`. Slicing is zero-copy (reference-counted). Tokio's
    I/O layer is already built on this.

15. **`SmallVec` for bounded collections.** Tool arguments, agent tool lists,
    message attachments, skill tags/triggers — collections that are almost always
    small (1-8 items). `SmallVec<[T; N]>` keeps them on the stack. Add `smallvec`
    to workspace deps.

16. **Zero-copy deserialization on I/O boundaries.** LLM JSON responses and
    WebSocket messages use `#[serde(borrow)]` with `Cow<'_, str>` fields when
    the input buffer outlives the parsed struct. Avoids per-field String allocation
    on the hot path.

17. **Protocol types in a separate crate.** `app/protocol` defines ClientMessage
    and ServerMessage. Both gateway and client depend on it. No circular dependency.

18. **CLI dual mode.** Direct mode embeds Runtime for local dev/testing (default).
    Gateway mode connects via walrus-client. Same REPL, different backend.

19. **Runner abstraction deferred.** DirectRunner and GatewayRunner are concrete
    structs first. A Runner trait is introduced only when both exist and the
    shared interface is known.
