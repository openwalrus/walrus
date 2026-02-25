# Walrus Platform Architecture

Single source of truth for architecture. Describes what exists today and
resolved design decisions. Plans live in [plan/](./plan/).

## Design Principles

**General-purpose libraries, platform-specific composition.** Library crates
solve specific problems without opinions. The Gateway wires them into
the Walrus platform (like tower/axum).

**walrus-core as the shared vocabulary.** Traits and types in one core crate
with minimal deps. Concrete implementations live in dedicated crates.

**Runtime as the intelligence layer.** Composes LLM providers, memory,
skills, and MCP tools into a unified agent execution engine.

**Trait-first extensibility.** One primary trait per subsystem. Smallest
surface that solves the problem.

### Performance

- **Static dispatch by default.** RPITIT for async traits, generics for
  composition, enum dispatch for multiple concrete types. No `dyn Trait`
  on hot paths.
- **`CompactString` for identity strings.** Up to 24 bytes inline. Agent
  names, tool names, model IDs, role labels.
- **`SmallVec` for bounded collections.** Tool args (4), agent tools (8),
  skill tags (4), attachments (4). Stack-allocated when small.
- **`Bytes`/`BytesMut` for I/O.** WebSocket frames and LLM streaming.
  Zero-copy slicing via reference counting.
- **Zero-copy deserialization.** `#[serde(borrow)]` with `Cow<'_, str>`
  on I/O boundaries.
- **Flat structures.** `Vec<T>` with index references over pointer trees.

---

## Workspace Layout

### Current Crates

| Crate | Path | Depends On | Purpose |
|-------|------|------------|---------|
| walrus-llm | crates/llm | — | LLM trait, Message, Tool, Config, StreamChunk |
| walrus-core | crates/core | walrus-llm | Agent, Chat, Memory, Embedder, Channel, Skill |
| walrus-deepseek | crates/llm/deepseek | walrus-llm | DeepSeek LLM provider |
| walrus-runtime | crates/runtime | walrus-core, walrus-llm, walrus-deepseek | Runtime, Provider, Handler, team composition |
| walrus-sqlite | crates/sqlite | walrus-core | SqliteMemory via SQLite + FTS5 |

### Planned Crates

| Crate | Path | Purpose | Phase |
|-------|------|---------|-------|
| walrus-telegram | crates/telegram | Telegram channel adapter | 2 |
| walrus-protocol | app/protocol | Wire types (ClientMessage, ServerMessage) | 3 |
| walrus-gateway | app/gateway | WebSocket server, sessions, auth, crons | 3 |
| walrus-client | app/client | WebSocket client library | 4 |
| walrus-cli | app/cli | CLI (direct + gateway modes) | 4 |
| walrus-hub | app/hub | Hub manifest types, registry, install/update | 6 |

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

Unified LLM interface. `LLM` trait with `send` and `stream` methods.
Provider-agnostic types: Message, Role, Response, StreamChunk, Tool,
ToolCall, ToolChoice, Config, General, Usage, FinishReason.
`estimate_tokens` for rough token counting.

---

## walrus-core

Shared vocabulary. Depends only on walrus-llm.

**Agent** — Config struct: name, description, system_prompt, tools
(`SmallVec<[CompactString; 8]>`), skill_tags (`SmallVec<[CompactString; 4]>`).
Builder-style API.

**Chat** — Session state: agent_name (`CompactString`) + message history.

**Memory trait** — Structured knowledge store. `get`, `set`, `remove`,
`entries`, `compile` (sync). `store`, `recall`, `compile_relevant` (async,
RPITIT). Bounds: `Send + Sync`. `&self` for mutations (interior mutability).
InMemory: `Mutex<Vec>`-backed default.

**MemoryEntry** — key (`CompactString`), value, metadata, timestamps,
access_count, optional embedding.

**RecallOptions** — limit, time_range, relevance_threshold.

**Channel trait** — `platform()`, `connect() -> impl Stream`, `send()`.
Platform enum (Telegram). ChannelMessage, Attachment types.

**Skill** — name, description, version, tier (SkillTier), tags, triggers,
tools, priority, body. Pure data struct.

**SkillTier** — Bundled < Managed < Workspace.

**Embedder trait** — `async fn embed(&self, text: &str) -> Vec<f32>`.

**with_memory** — Helper appending `memory.compile()` to system prompt.

---

## walrus-sqlite

Persistent Memory backend using SQLite with FTS5 full-text search.

**`SqliteMemory<E: Embedder>`** — Generic over Embedder for optional vector
search. Wraps `rusqlite::Connection` in `Mutex`. Uses `bundled` feature
(no system SQLite dependency).

**Schema** — `memories` table (key TEXT PK, value, metadata JSON, timestamps,
access_count, embedding BLOB). `memories_fts` FTS5 virtual table with
AFTER INSERT/UPDATE/DELETE triggers for sync.

**CRUD** — Implements Memory trait. `get()` updates access tracking on each
read. `set()` uses `ON CONFLICT(key) DO UPDATE` to preserve `created_at`.
`store_with_metadata()` for metadata + embedding storage.

**Recall pipeline** — BM25 via FTS5 MATCH, temporal decay (30-day half-life
from `accessed_at`), time_range and relevance_threshold filtering, MMR
re-ranking (Jaccard similarity, lambda 0.7), top-k truncation (default 10).

**compile_relevant** — recall(limit 5), format as `<memory>` XML blocks.

---

## walrus-runtime

Central composition point. `Runtime<M: Memory = InMemory>`.

**Provider** — Enum dispatch over LLM implementations (DeepSeek).

**Tool dispatch** — BTreeMap registry. `register()` + `dispatch()`. Auto-loops
up to 16 rounds. Auto-registers "remember" tool when memory is present (DD#23).

**SkillRegistry** — Loads TOML-frontmatter skill files from a directory
(`load_dir`). Indexes by tag/trigger. Ranks by tier then priority.

**McpBridge** — Connects to MCP servers via rmcp. Converts tool definitions,
dispatches calls through the protocol.

**Memory integration** — `compile_relevant()` injects memories into system
prompts. Memory flush before compaction (DD#7). Tool glob resolution (DD#21).

**Compaction** — Per-agent functions trimming message history at 80% context.

**Team composition** — `build_team()` registers workers as tools on a leader.

---

## Implementation Phases

- **[Phase 1: Core](./plan/phase1-core.md)** — Performance primitives, Memory
  trait revision, Channel/Skill/Embedder traits, walrus-sqlite.
- **[Phase 2: Runtime](./plan/phase2-runtime.md)** — SkillRegistry, McpBridge,
  memory/skills integration, Telegram adapter.
- **[Phase 3: Gateway](./plan/phase3-gateway.md)** — Protocol types, sessions,
  auth, WebSocket server, channel routing, cron, binary entry point.
- **[Phase 4: CLI & Client](./plan/phase4-cli.md)** — Client library, CLI with
  direct/gateway modes, REPL, management commands.
- **[Phase 5: Workspace + OpenAPI](./plan/phase5-workspace.md)** — Workspace
  directory concept, `walrus init`, REST endpoints with SSE, OpenAPI spec
  generation via utoipa, Swagger UI, example workspace.
- **[Phase 6: Hub](./plan/phase6-hub.md)** — GitHub-based resource registry.
  TOML manifests in a central repo, `walrus hub` CLI commands for search,
  install, update. Homebrew-style taps for custom sources.

---

## Design Decisions

1. **Embedder in walrus-core.** walrus-sqlite needs it for optional vector
   search without depending on walrus-runtime.

2. **Channel adapters use reqwest directly.** No teloxide dependency.

3. **Channel-to-agent routing.** Gateway routing table: exact (platform +
   channel_id), platform catch-all, default agent fallback.

4. **Skill trigger strategy.** Keyword matching. No ML dependency.

5. **Memory flush strategy.** Tool-based ("remember" command via
   Memory::store). Explicit user control.

6. **Cron isolation.** Config-only, no persistence. Fresh session per run.

7. **Memory flush before compaction.** Runtime orchestrates: silent LLM turn,
   Memory::store, then Compactor trims.

8. **Session scoping.** Gateway owns Session. Scopes: Main (full), Dm
   (per-peer), Group (per-group), Cron (per-job, fresh each run).

9. **Tool access control.** Agent tools field = allowlist. Gateway adds deny
   layer for untrusted scopes.

10. **DM safety.** Gateway ignores unknown senders by default.

11. **No dyn dispatch for core traits.** RPITIT + generics. Enum dispatch
    (Provider, MemoryBackend) for multiple concrete types.

12. **Skill config format.** TOML frontmatter (+++delimited).

13. **`CompactString` for identity strings.** Up to 24 bytes inline.

14. **`Bytes` for I/O buffers.** Zero-copy WebSocket frames and streaming.

15. **`SmallVec` for bounded collections.** Stack-allocated when small.

16. **Zero-copy deserialization on I/O boundaries.** `#[serde(borrow)]`.

17. **Protocol types in separate crate.** `app/protocol` shared by gateway
    and client.

18. **CLI dual mode.** Direct mode (embedded Runtime) and gateway mode
    (walrus-client WebSocket).

19. **Runner abstraction deferred.** Concrete structs first, trait in P4-06.

20. **MCP server lifecycle.** Gateway spawns processes from config, passes
    rmcp peers to McpBridge.

21. **Tool glob patterns.** `"browser_*"` expands against registered tool
    names. No-match logs warning.

22. **MemoryBackend enum dispatch.** Gateway selects backend from `[memory]`
    config. Wraps InMemory and SqliteMemory. Monomorphizes Runtime.

23. **"remember" tool auto-registration.** Runtime auto-registers when memory
    is present. Schema: `{ key: string, value: string }`.

24. **Workspace directory layout.** A walrus project is a directory with
    `walrus.toml` at the root. Subdirectories: `agents/` (per-agent TOML),
    `skills/` (Markdown with TOML frontmatter), `mcp/` (per-server TOML),
    `cron/` (per-job TOML), `data/` (runtime state, gitignored). Single-file
    mode (`walrus.toml` with inline `[[agents]]`) still supported.

25. **REST + WebSocket dual transport.** Gateway serves REST (`/v1/*`) and
    WebSocket (`/ws`) simultaneously. REST uses standard HTTP + SSE for
    streaming. Protocol types shared between both transports.

26. **utoipa for OpenAPI generation.** Code-first: `#[derive(ToSchema)]` on
    protocol types, `#[utoipa::path]` on handlers. Spec generated at compile
    time, served at `/api-docs/openapi.json`. Optional `openapi` feature flag
    on walrus-protocol to avoid pulling utoipa unconditionally.

27. **Hub crate in `app/hub`.** Application-level crate defining hub metadata
    structure. `app/cli` imports types for git-based operations.

28. **Hub manifest format.** One TOML file per resource: name, version,
    resource_type (skill/agent/mcp/workspace), author, tags, `[source]`
    table with github repo, path, and ref.

29. **Hub repo layout.** Type-prefixed directories in the hub GitHub repo:
    `skills/`, `agents/`, `mcp/`, `workspaces/`.

30. **Hub install targets.** Resources install to workspace directories with
    `SkillTier::Managed` tier. Workspace-tier files take priority.

31. **Version pinning via lockfile.** `hub.lock` at workspace root records
    installed resources with resolved commit SHA. No semver resolution.

32. **Multiple hub sources (taps).** Default: `walrus-hub/hub`. Custom repos
    via `walrus hub add-source`. Configurable in `~/.walrus/config.toml`.

33. **Local index cache.** Hub repos shallow-cloned to `~/.walrus/hub/`.
    Search operates locally. `walrus hub update` pulls latest.

34. **No inter-resource dependencies.** Hub resources are independent.

35. **Shell out to git.** Like Homebrew — reuses SSH keys and credential
    helpers. No `git2` crate dependency.
