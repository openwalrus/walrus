# Walrus Platform Architecture

This document describes the design for extending Walrus from an agent library into a
full agent platform. It introduces channels, persistent memory, skills, and MCP support
and explains how each maps onto Walrus's existing abstractions.

The goal is to let any Walrus agent talk to real users on real messaging platforms, remember
context across sessions, and acquire new capabilities at runtime through modular skill
packages — all while keeping the Rust-native design philosophy that makes Walrus what it is.

## Design Principles

Every decision in this document is filtered through a small set of invariants.

**General-purpose libraries, platform-specific composition.** The relationship between the
library crates and the Gateway is analogous to tower and axum. The library crates solve
specific problems (talking to LLMs, defining agents, connecting to messaging platforms)
without any opinion about how they are composed. The Gateway is the opinionated layer that
wires them into the Walrus platform. A developer building a different agent framework, a
CLI tool, or a custom service can pull in any library crate independently.

**walrus-core as the shared vocabulary.** All fundamental traits and types — Agent, Chat,
Memory, Channel, Skill — live in a single core crate. This avoids a proliferation of tiny
crates for concepts that are simple and closely related. walrus-core has minimal
dependencies — no database drivers, no platform SDKs. Concrete implementations (SQLite
memory, Telegram adapter, skill registry) live in dedicated crates.

**Runtime as the intelligence layer.** The Runtime composes LLM providers, persistent
memory, skills, and MCP tools into a unified agent execution engine. Any application that
depends on walrus-runtime gets memory-augmented, skill-enhanced, MCP-capable agents —
without needing the Gateway. The Runtime is the brain; the Gateway is the face.

**Trait-first extensibility.** Each subsystem introduces one primary trait. Concrete
implementations are just one side of that trait; anyone can bring their own.

**Workspace discipline.** New crates inherit all dependency versions from the workspace root.
No version strings appear in member Cargo.toml files. File conventions (file-level doc
comments, no `super::` imports, `mod` after `use`) carry forward into every new crate.

**No over-abstraction.** Each new trait has the smallest surface that solves the problem.
If something only needs to be a struct, it stays a struct.

---

## Workspace Layout

### Current Crate Map

- **walrus-llm** — leaf crate. LLM trait, Config trait, Message, Tool, ToolCall,
  StreamChunk, Response, General config.
- **walrus-agent** — depends on walrus-llm. Agent config, Chat session, Memory trait,
  InMemory implementation.
- **walrus-deepseek** — depends on walrus-llm. DeepSeek LLM provider.
- **walrus-runtime** — depends on all above. Runtime orchestrator, Provider enum, Handler,
  Compactor, team composition.

### Proposed Changes

**Rename walrus-agent to walrus-core.** The crate already serves as the foundation for
agent-related abstractions. Renaming it to walrus-core reflects its expanded role as the
shared vocabulary for the entire workspace. It gains the Channel trait, Skill struct, and
an expanded Memory trait alongside the existing Agent, Chat, and Skill types.

**Heavy drivers get their own crates.** SQLite memory, Telegram, and Discord each carry
substantial platform-specific dependencies (rusqlite, reqwest, twilight) that should not
be forced on walrus-runtime users. Each lives in its own crate, depending only on
walrus-core for traits.

**Fold lightweight logic into walrus-runtime.** The skill registry is a straightforward
module — a few hundred lines with only serde_yaml as a new dependency. It lives inside
walrus-runtime rather than justifying its own crate.

The resulting workspace:

| Crate | Path | Depends On | Purpose |
|-------|------|------------|---------|
| walrus-llm | crates/llm | — | LLM trait, Message, Tool, Config, StreamChunk |
| walrus-core | crates/core | walrus-llm | Agent, Chat, Memory, Channel, Skill, Embedder traits and types |
| walrus-deepseek | crates/llm/deepseek | walrus-llm | DeepSeek LLM provider |
| walrus-sqlite | crates/sqlite | walrus-core | SqliteMemory — Memory backed by SQLite + FTS5 |
| walrus-runtime | crates/runtime | walrus-core, walrus-llm, walrus-deepseek, rmcp | Runtime, SkillRegistry, MCP bridge |
| walrus-telegram | crates/telegram | walrus-core | Telegram channel adapter |
| walrus-discord | crates/discord | walrus-core | Discord channel adapter |
| walrus-gateway | crates/gateway | walrus-runtime, walrus-sqlite, walrus-telegram, walrus-discord | WebSocket server, sessions, auth, crons |

```text
  platform    ┃             walrus-gateway
  ━━━━━━━━━━━━╋━━━━━━━/━━━━━|━━━━━\━━━━━\━━━━━━
  runtime     ┃   walrus-runtime   |     |  sqlite
              ┃  / |    |    \   tgram discord |
  core        ┃ llm | dseek rmcp  |     |   |
              ┃   core             core  core core
              ┃    |
              ┃   llm
```

---

## walrus-core

walrus-core (renamed from walrus-agent) is the shared vocabulary of the workspace. It
defines the fundamental traits and types that every other crate builds on. It depends only
on walrus-llm.

### Existing Types

- **Agent** — config struct: name, description, system_prompt, tools. Gains an optional
  skill_tags field.
- **Chat** — session state: agent_name, messages.

### Revised: Memory Trait

The existing Memory trait is synchronous, key-value only, with a compile method. The new
design replaces it with a single unified Memory trait that covers both simple in-memory
usage and persistent storage with search.

Methods: get, set, remove (async, key-value basics), store (upsert with optional embedding
and metadata), recall (query with RecallOptions, returns ranked MemoryEntry values),
compile (compile all entries for prompt injection), compile_relevant (query-aware
compilation that selects the most relevant entries).

Default implementations of store, recall, and compile_relevant are provided so that simple
backends only need to implement get/set/remove/compile. InMemory continues to work as
before — it implements the basic methods and inherits no-op defaults for the search methods.

### New: Channel Trait

The Channel trait defines bidirectional communication with a messaging platform. It has
two associated types: Event (platform-specific inbound events) and ChannelConfig
(credentials and settings). Three methods: connect (returns a Stream of Events), send
(delivers an outbound ChannelMessage), and platform (returns a Platform enum variant).

### New: ChannelMessage and Platform

ChannelMessage is the normalized message envelope: platform, channel ID, sender ID, text
content, attachments, optional reply-to, and timestamp. The Attachment struct carries kind
(image, file, audio, video), URL, and optional name. The Platform enum starts with Telegram
and Discord variants.

A From implementation converts ChannelMessage to llm::Message and back.

### New: Skill Struct

The Skill struct represents a parsed Markdown skill: name, description, version, tags,
triggers, tools, priority, and body. This is a data type only — parsing and registry logic
live in walrus-runtime.

### New: Embedder Trait

Single async method: takes a text string, returns a Vec of f32. Used by the memory
retrieval pipeline for vector search. Lives in walrus-core so any crate can implement it.

### New: MemoryEntry and RecallOptions

MemoryEntry: key, value, metadata (JSON), created_at, accessed_at, access_count, optional
embedding. RecallOptions: limit, time_range, relevance_threshold.

---

## Memory System

The Memory trait is defined in walrus-core with no storage dependencies. The concrete
SQLite implementation — SqliteMemory — lives in its own crate, walrus-sqlite, which
depends only on walrus-core and rusqlite. This keeps walrus-core lightweight and lets
users swap in a different storage backend without pulling in SQLite. InMemory remains
in walrus-core as a zero-dependency default for simple use cases.

### SQLite Schema

A memories table: id, key (unique), value, metadata (nullable JSON), embedding (nullable
blob), created_at, accessed_at, access_count. A memories_fts virtual table (FTS5) over
key and value for BM25 search. An index on accessed_at for temporal queries.

### Retrieval Pipeline

1. **BM25 search** — query against FTS5, get keyword-relevant entries with BM25 scores.
2. **Vector search** — if embeddings are available, compute cosine similarity between
   query embedding and stored embeddings.
3. **Score fusion** — combine via Reciprocal Rank Fusion (configurable weight split,
   default 30% BM25 / 70% vector; 100% BM25 when no embeddings).
4. **Temporal decay** — exponential decay based on accessed_at (configurable half-life,
   default 30 days). Entries marked persistent are exempt.
5. **MMR diversity** — Maximal Marginal Relevance to reduce redundancy (configurable
   lambda, default 0.7).
6. **Return top-k** — truncate to requested limit.

### Embedding Strategy

SqliteMemory holds an optional Embedder. Without one, retrieval falls back to BM25-only —
vector search is a progressive enhancement.

### Auto-Extraction

After a conversation turn, the Runtime can store new facts via an explicit "remember" tool
(calls Memory::store) or via a post-processing LLM call. The tool-based approach is simpler
and gives the model explicit control.

---

## Skill System

The Skill struct is defined in walrus-core. The SkillRegistry — which handles parsing,
discovery, and selection — lives in walrus-runtime as a module.

### Skill Format

A Markdown file with YAML frontmatter. Frontmatter fields: name, description, version,
tags, triggers (regex or keywords), tools (required tool names), priority (0-255). The body
is Markdown prose injected into the system prompt when active.

### Three-Tier Discovery

1. **Bundled** — compiled into the binary at build time. Lowest priority.
2. **Managed** — loaded from ~/.walrus/skills/ at startup. Overrides bundled.
3. **Workspace** — loaded from .walrus/skills/ in the project directory. Highest priority.

Resolution by name: workspace overrides managed overrides bundled.

### Selective Injection

1. **Trigger matching** — test user message against each skill's trigger patterns.
2. **Tag filtering** — constrain by the agent's skill_tags.
3. **Priority sorting** — descending; workspace > managed > bundled on ties.
4. **Token budgeting** — accept skills in priority order until budget is exhausted.

Selected skill bodies are wrapped in a structured block and injected into the agent's
system prompt. Skill-declared tools are added to the agent's tool set for the request.

---

## MCP Integration

MCP support is provided through rmcp, the official Rust MCP SDK. walrus-runtime depends
on rmcp directly and provides a thin bridge to the Runtime's tool registry.

### The Bridge

One function: takes a mutable Runtime reference and an rmcp client handle, iterates over
discovered tools, calls Runtime::register for each. The handler for each tool is a closure
that forwards calls through rmcp. Transport setup (stdio, HTTP+SSE, WebSocket) is handled
entirely by rmcp.

After registration, MCP tools are indistinguishable from local tools — same BTreeMap, same
Handler dispatch, same agent experience.

---

## Runtime

The Runtime is the central composition point. It composes LLM providers, persistent memory,
skills, MCP tools, and tool dispatch into a unified agent execution engine.

### What the Runtime Composes

- **LLM providers** (existing) — sending messages to language models, streaming responses.
- **Tool dispatch** (existing) — registering and invoking tools via type-erased handlers.
- **Agent configs** (existing) — managing named agents with system prompts and tool lists.
- **Team composition** (existing) — building multi-agent teams via tool delegation.
- **Persistent memory** (new) — accepts any Memory implementation. Queries relevant
  memories and injects them into agent system prompts before each LLM call.
- **Skill injection** (new) — SkillRegistry module. Selects relevant skills and injects
  their content into agent system prompts before each LLM call.
- **MCP tools** (new) — rmcp bridge module. Connects to MCP servers and registers their
  tools into the existing tool registry.

### How Memory and Skills Integrate

The Runtime holds an optional boxed Memory and an optional SkillRegistry. When neither is
configured, it behaves exactly as it does today.

When memory is configured, the send and stream methods gain a pre-processing step: query
Memory::compile_relevant with the current user message, inject the result into the agent's
system prompt.

When skills are configured, the same pre-processing step queries SkillRegistry::select
with the user message and agent's skill tags. Selected skill bodies are injected into the
system prompt, and skill-declared tools are added to the agent's tool set.

Both injections happen on a cloned agent config — the original is not mutated. The clone
is used for one request and discarded.

### New Runtime Modules

- **skill.rs** — SkillRegistry: parsing, discovery, selection. Depends on serde_yaml.
- **mcp.rs** — rmcp bridge function. Depends on rmcp.

### New Public Methods

- set_memory — configures persistent memory (accepts any Memory implementation).
- set_skills — configures the skill registry.
- register_mcp — registers tools from an MCP server via rmcp.

Existing methods (send, stream, send_to, register, chat, add_agent) remain unchanged in
their signatures.

---

## Gateway

The Gateway is the application shell. It sits on top of the Runtime and adds channels,
sessions, authentication, scheduled tasks, and a WebSocket protocol. It does not implement
any agent intelligence — that is the Runtime's job.

### Architecture

The Gateway depends on walrus-runtime (which includes skills and MCP), walrus-sqlite (the
default memory backend), and the channel adapter crates. It produces a binary at
crates/gateway/src/bin/main.rs.

The Gateway struct holds:
- A Runtime instance (already configured with memory, skills, and MCP tools).
- Registered channel adapters (map from Platform to adapter).
- A session registry (BTreeMap of session ID to session state).
- A cron scheduler (registered jobs and their evaluation loop).

### Walrus Protocol

JSON over WebSocket. Three message categories:

- **Requests** (client → gateway): authenticate, send_message, subscribe, unsubscribe, ping,
  cron_add, cron_remove, cron_enable, cron_disable, cron_list.
- **Responses** (gateway → client): ok payload or error code/message, referencing request ID.
- **Events** (gateway → client, unsolicited): message_chunk (streaming), channel_event,
  session_expired, cron_fired (job name, agent, result summary).

### Session Management

A session maps a client (or channel sender) to an Agent and Chat. Sessions track: ID,
agent name, Chat instance, role, originating platform, last activity. Configurable TTL
with memory flush on expiry.

### Authentication

Three roles: Admin (manage everything), User (chat), Channel (automated adapter).
Token-based via an Authenticator trait. Default implementation validates API keys;
alternative implementations can use JWT, OAuth, etc.

### End-to-End Message Flow

1. **Resolve session** from request ID or channel/sender pair.
2. **Execute** via Runtime::stream or Runtime::send. The Runtime handles memory, skills,
   and tool dispatch internally.
3. **Deliver outbound** — if from a channel, convert response to ChannelMessage and send
   via the adapter.

### Channel Routing

Maps channel adapters to agents. Configurable per-platform, per-channel-ID, or per-sender.

### Cron System

Crons are the Gateway's scheduler — they let agents act on their own initiative rather than
only responding to inbound messages. A cron is a named, recurring job that triggers agent
execution on a schedule.

**CronJob definition.** Each job specifies: a name, a cron expression (standard 5-field or
extended 6-field with seconds), the target agent name, a prompt (the message sent to the
agent when the job fires), an optional output channel (Platform + channel ID to deliver the
result), and an enabled flag.

**Scheduler.** The Gateway runs a scheduler loop that evaluates registered cron expressions
against the current time. When a job fires, the scheduler creates a transient session,
sends the prompt to the target agent via Runtime::send or Runtime::stream, and — if an
output channel is configured — delivers the response through the appropriate channel
adapter. If no output channel is set, the result is logged or published as a WebSocket
event for connected clients.

**Registration.** Jobs can be registered statically at Gateway startup (from configuration)
or dynamically via the WebSocket protocol (Admin role only). Dynamic registration supports
add, remove, enable, disable, and list operations.

**Concurrency.** Each fired job runs as an independent tokio task. A configurable max
concurrency limit prevents runaway scheduling. If a job is still running when its next tick
arrives, the tick is skipped (no overlap). Long-running jobs are subject to a configurable
timeout.

**Use cases.** Daily digests, periodic data pulls, health checks, proactive notifications,
batch processing, content generation on a schedule — any scenario where an agent should
act without waiting for a user message.

---

## Modifications to Existing Crates

### walrus-llm

No changes.

### walrus-core (was walrus-agent)

Renamed. The Memory trait is expanded from sync key-value to async with search (store,
recall, compile_relevant with defaults). InMemory continues to work — it inherits no-op
defaults for the search methods. Gains: Channel trait, ChannelMessage, Platform enum,
Attachment, Skill struct, Embedder trait, MemoryEntry, RecallOptions. Agent gains optional
skill_tags field.

### walrus-runtime

Gains two new modules: skill.rs (SkillRegistry), mcp.rs (rmcp bridge). Runtime struct
gains optional memory (boxed Memory trait object) and skill fields with setter methods.
New dependencies: serde_yaml, rmcp. Existing public API unchanged. No SQLite dependency —
the Runtime works against the Memory trait.

### walrus-deepseek

No changes unless a DeepSeek embeddings endpoint is added.

---

## New Traits and Types

### In walrus-core

**Traits:** Memory (expanded), Channel, Embedder.

**Structs:** ChannelMessage, Attachment, Skill, MemoryEntry, RecallOptions.

**Enums:** Platform (Telegram, Discord), AttachmentKind (Image, File, Audio, Video),
SkillTier (Bundled, Managed, Workspace).

### In walrus-sqlite

**Structs:** SqliteMemory (implements Memory).

### In walrus-runtime

**Structs:** SkillRegistry.

### In walrus-gateway

**Traits:** Authenticator.

**Structs:** Gateway, Session, CronJob, CronScheduler.

**Enums:** GatewayRole (Admin, User, Channel), ClientMessage, ServerMessage.

---

## Dependency Strategy

### New External Dependencies

All in workspace root Cargo.toml, inherited with workspace = true.

| Dependency | Used By | Purpose |
|------------|---------|---------|
| rusqlite (features: bundled) | walrus-sqlite | SQLite with FTS5 |
| chrono | walrus-sqlite | Timestamps and temporal decay |
| rmcp | walrus-runtime | Official Rust MCP SDK |
| serde_yaml | walrus-runtime | YAML frontmatter parsing |
| twilight-gateway | walrus-discord | Discord gateway WebSocket |
| twilight-http | walrus-discord | Discord REST API |
| axum | walrus-gateway | HTTP/WebSocket server |
| tokio-tungstenite | walrus-gateway | WebSocket |
| jsonwebtoken | walrus-gateway | JWT auth (optional) |
| cron | walrus-gateway | Cron expression parsing |
| uuid (features: v4) | walrus-gateway | Session IDs |

### Feature Flags

Channel adapters as optional features of walrus-gateway. Vector search (requiring an
embedder) as an optional feature of walrus-runtime.

---

## Implementation Phases

### Phase 1: Core and Runtime

1. Rename walrus-agent to walrus-core. Expand Memory trait with async search methods
   (store, recall, compile_relevant with defaults). Add Channel, Skill, Embedder traits
   and associated types.
2. walrus-sqlite — SqliteMemory implementing Memory with FTS5 BM25 search and temporal
   decay.
3. Add skill.rs to walrus-runtime — SkillRegistry with YAML parsing and three-tier
   filesystem discovery.
4. Add mcp.rs to walrus-runtime — rmcp bridge function.
5. Integrate memory (trait-based) and skills into Runtime::send and Runtime::stream as
   pre-processing.

### Phase 2: Channel Adapters

6. walrus-telegram — Telegram adapter with long-polling and sendMessage via reqwest.
7. walrus-discord — Discord adapter with twilight-gateway and twilight-http.
8. Add vector search and MMR diversity to walrus-sqlite.

### Phase 3: Gateway

9. walrus-gateway — WebSocket server, sessions, auth, channel routing.
10. Cron scheduler — job registration, evaluation loop, output delivery.
11. Binary entry point at crates/gateway/src/bin/main.rs.

### Phase 4: Polish

12. Token budgeting for skill injection.
13. Memory auto-extraction.
14. Integration tests with mock channels, mock MCP servers, cron scheduler, and in-memory
    SQLite.

---

## Open Questions

1. **Embedder placement.** The Embedder trait starts in walrus-core. If only walrus-runtime
   needs it, it could move there to keep walrus-core even leaner.

2. **Channel adapter dependencies.** Telegram: teloxide (heavier, more features) or direct
   reqwest (lighter, more control)? Recommendation: direct reqwest.

3. **Gateway vs. server crate.** Binary in walrus-gateway or a separate walrus-server?
   Recommendation: walrus-gateway for simplicity.

4. **Channel-to-agent routing.** Per-agent, per-channel, or separate routing table?

5. **Skill trigger strategy.** Regex, keywords, or embedding similarity? Recommendation:
   regex with keyword shorthand.

6. **Memory flush strategy.** On session expiry: full flush, LLM-summarized extraction, or
   tool-based? Recommendation: tool-based ("remember" tool gives the model control).

7. **Cron persistence.** Should dynamically registered cron jobs be persisted to SQLite so
   they survive restarts, or is configuration-only sufficient? Recommendation: persist to
   SQLite for production use, with config-file-only as the simpler starting point.
