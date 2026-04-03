# 0135 - Agent-First Protocol

- Feature Name: Agent-First Protocol
- Start Date: 2026-04-03
- Discussion: [#135](https://github.com/crabtalk/crabtalk/issues/135)
- Crates: core, runtime, daemon, cli, gateway
- Supersedes: [0064 (Session)](0064-session.md), [0078 (Compact Session)](0078-compact-session.md)
- Updates: [0018 (Protocol)](0018-protocol.md), [0038 (Memory)](0038-memory.md)

## Summary

Replace session-centric protocol addressing with agent-centric addressing.
Users talk to agents, not sessions. Introduce guest turns for multi-agent
conversations and compaction archives as the agent's long-term memory.

## Motivation

The original protocol was session-centric: clients managed session IDs to kill,
reply, compact, and route messages. This leaked an implementation detail (the
session ID) into every client and forced multi-agent interaction into either
permanent agent switching or invisible delegation.

Problems with the session model:

1. **Session IDs leak everywhere.** Every client (CLI, Telegram, WeChat, IDE)
   must track session IDs to route replies, kill conversations, and handle
   ask_user prompts. If a client loses the ID, the conversation is orphaned.

2. **Multi-agent is invisible.** When agent A delegates to agent B, the result
   comes back as a tool result string. The user hears A's summary of B's
   answer, never B's actual voice. There's no multi-agent conversation.

3. **Session ≠ conversation.** "Session" conflated device connections (CWD,
   transport state) with agent memory (message history, compaction). These are
   different lifecycles — connections are ephemeral, conversations persist.

## Design

### Core model

Each agent has **one continuous conversation** per user. Conversations are
keyed by `(agent, sender)` — no session IDs in the protocol.

```
Client: StreamMsg { agent: "crab", content: "hello", sender: "user" }
Daemon: resolves (crab, user) → internal conversation, runs agent, streams response
```

### Conversation vs session

| | Session | Conversation |
|---|---|---|
| What | Device ↔ daemon connection | Agent's memory with a user |
| Key | connection/device ID | (agent, sender) |
| Lifetime | ephemeral | persistent |
| State | CWD, transport | messages, title, JSONL, archives |

Sessions are daemon-internal. Conversations are the protocol-visible abstraction.

### Protocol changes

Client messages address conversations by `(agent, sender)`:

```protobuf
message StreamMsg {
  string agent = 1;
  string content = 2;
  optional string sender = 4;
  optional string cwd = 5;
  optional string guest = 6;  // guest turn
}

message KillMsg {
  string agent = 1;
  string sender = 2;
}

message ReplyToAsk {
  string agent = 1;
  string sender = 2;
  string content = 3;
}

message CompactMsg {
  string agent = 1;
  string sender = 2;
}
```

Removed from the protocol: `session` (u64 ID), `new_chat`, `resume_file`.

Server responses no longer include session IDs:

```protobuf
message StreamStart {
  string agent = 1;  // no session field
}
```

### Guest turns

The `guest` field on `StreamMsg` enables multi-agent conversations. When set,
the daemon runs the guest agent against the primary agent's conversation
history — text-only, no tool dispatch.

Flow:
1. Client sends `StreamMsg { agent: "twin", content: "question", guest: "crab" }`
2. Daemon finds twin's conversation
3. Adds user message to twin's history
4. Injects guest framing (auto-injected system message)
5. Runs crab against twin's history with crab's system prompt (no tools)
6. Tags response with `agent: "crab"`
7. Appends to twin's history

The guest's response appears as a first-class message in the conversation,
attributed to the guest. No delegation, no tool results, no paraphrasing.

### Bidirectional framing

Both guest and primary need context about multi-agent conversation:

- **Guest framing** (injected when a guest runs): "You are joining a
  conversation as a guest. Messages prefixed with [agent_name] are from other
  agents."
- **Primary framing** (injected when the primary runs and guest messages exist
  in history): "Messages prefixed with [agent_name] are from guest agents.
  Continue responding as yourself."

Both are `auto_injected` — stripped before each run, re-injected fresh. Zero
accumulation.

### Message attribution

The `Message` struct gains an `agent` field:

```rust
#[serde(default, skip_serializing_if = "String::is_empty")]
pub agent: String,
```

Empty = the conversation's primary agent. Non-empty = a guest. When building
LLM requests, assistant messages with non-empty `agent` are prefixed with
`[agent_name]:` so every agent can distinguish speakers.

`Message::with_agent_prefix()` handles the prefixing — one function, used by
both `build_request` and `guest_stream_to`.

### Compaction as memory

Compaction markers become archive boundaries. Each compact marker stores a
title (first sentence of the summary, max 60 chars) and a timestamp:

```json
{"compact":"Summary of pricing discussion...","title":"Pricing analysis for solo dev tools.","archived_at":"2026-04-03T10:00:00Z"}
```

The conversation is continuous — compaction doesn't create a new conversation,
it archives a segment of the existing one. Archived segments are browsable
via `Conversation::load_archives()` and available to the recall tool as
long-term memory.

```
Crab's memory:
├── [active] Current conversation
├── "Pricing analysis for solo dev tools." — 2 days ago
├── "Auth module refactor plan." — 5 days ago
└── "HN competitor signal analysis." — last week
```

### What dies

- **Session IDs in the protocol** — replaced by (agent, sender)
- **`new_chat`** — the conversation is continuous, compaction handles the window
- **`resume_file`** — one conversation per (agent, user), always active
- **Client-side @mention logic** ([0078](0078-compact-session.md)) — guest turns
  handle it daemon-side
- **Session forking** — agents are the abstraction, not sessions

## Supersedes

### 0064 - Session

The session model is replaced by conversations. The JSONL file format is
preserved (backward compatible with added `title` and `archived_at` fields on
compact markers, and `agent` field on messages). The `Session` struct is renamed
to `Conversation`. Session IDs are removed from the protocol.

### 0078 - Compact Session

The compact-then-handoff pattern for @mentions is replaced by guest turns.
The daemon handles multi-agent conversation natively — no client-side compact
logic needed.

## Updates

### 0018 - Protocol

Session-addressed messages are replaced with (agent, sender) addressing.
`StreamMsg` and `SendMsg` gain a `guest` field. `SessionInfo` becomes
`ActiveConversationInfo`. See protocol changes section above.

### 0038 - Memory

Compaction archives become the primary long-term memory mechanism. The
recall tool searches across archived segments. See [#101 (revised)](https://github.com/crabtalk/crabtalk/issues/101) for
the pluggable memory provider aligned with this model.
