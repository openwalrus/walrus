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

## Dependency Graph

```text
P1-04 ─→ P2-01 ─→ P2-04
                          P2-02 (independent)
P1-01, P1-02 ─→ P2-03
P1-03 ─→ P2-05 ─→ P2-06
```

P2-02, P2-03, and P2-05 are independent of each other.

## Workspace Changes

Add to `[workspace.dependencies]`:

```toml
rmcp = "0.16"
serde_yaml = "0.9"
toml = "0.8"
```

Add `crates/telegram` to workspace members.

## Completion Checklist

- [ ] All 6 units complete
- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] Runtime with memory/skills injects into system prompts correctly
- [ ] Runtime without memory/skills behaves identically to before
- [ ] `docs/src/design.md` updated
