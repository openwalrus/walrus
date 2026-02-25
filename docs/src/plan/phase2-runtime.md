# Phase 2: Runtime

Integrate memory, skills, and MCP into the Runtime, and implement the Telegram
channel adapter.

## Unit Index

| Unit | Title | Depends On |
|------|-------|------------|
| [P2-01](./units/P2-01.md) | Add SkillRegistry to walrus-runtime | Phase 1 (P1-04) |
| [P2-02](./units/P2-02.md) | Add MCP bridge to walrus-runtime | — |
| [P2-03](./units/P2-03.md) | Integrate memory into Runtime (`Runtime<M: Memory>`, memory flush before compaction) | Phase 1 (P1-01, P1-02) |
| [P2-04](./units/P2-04.md) | Integrate skills into Runtime | P2-01 |
| [P2-05](./units/P2-05.md) | Create walrus-telegram crate | Phase 1 (P1-03) |
| [P2-06](./units/P2-06.md) | Implement Telegram Channel (connect + send) | P2-05 |
| [P2-07](./units/P2-07.md) | Team delegation (worker agents call LLM) | P2-03, P2-04 |
| [P2-08](./units/P2-08.md) | Hook trait, compaction, ergonomic API | P2-07 |

## Dependency Graph

```text
P1-04 ─→ P2-01 ─→ P2-04 ─→ P2-07 ─→ P2-08
                          P2-02 (independent)
P1-01, P1-02 ─→ P2-03 ─→ P2-07
P1-03 ─→ P2-05 ─→ P2-06
```

P2-02, P2-03, and P2-05 are independent of each other.

## Workspace Changes

Add to `[workspace.dependencies]`:

```toml
rmcp = { version = "0.16", features = ["client", "transport-child-process"] }
serde_yaml = "0.9"
toml = "0.8"
```

Add `crates/telegram` to workspace members (auto-included via `crates/*` glob).

## Deviations

- **rmcp features**: Added `client` and `transport-child-process` features
  (spec said "0.16+" but didn't specify features).
- **P2-05 + P2-06 combined**: Implemented TelegramChannel skeleton and full
  connect/send in a single pass since they were sequential.
- **tokio as runtime dep**: Moved tokio from dev-dependencies to dependencies
  in walrus-runtime since McpBridge needs `tokio::process::Command` and
  `tokio::sync::Mutex` at runtime.

## Completion Checklist

- [x] All 8 units complete
- [x] `cargo check --workspace` and `cargo clippy --workspace` pass
- [x] `cargo test --workspace` passes (87 tests)
- [x] Runtime with memory/skills injects into system prompts correctly
- [x] Runtime without memory/skills behaves identically to before
- [x] Team delegation uses real LLM send loops (no stubs)
- [x] Hook trait replaces old Compactor, automatic compaction wired in
- [x] Re-exports and prelude module available
- [x] `docs/src/design.md` updated
