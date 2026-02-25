# Phase 1: Core

Expand walrus-core with new traits and types, and create walrus-sqlite.

## Unit Index

| Unit | Title | Depends On |
|------|-------|------------|
| [P1-01](./units/P1-01.md) | Revise Memory trait | — |
| [P1-02](./units/P1-02.md) | Add MemoryEntry and RecallOptions types | P1-01 |
| [P1-03](./units/P1-03.md) | Add Channel trait and types | — |
| [P1-04](./units/P1-04.md) | Add Skill struct and SkillTier | — |
| [P1-05](./units/P1-05.md) | Add Embedder trait | — |
| [P1-06](./units/P1-06.md) | Update Agent struct (skill_tags) | — |
| [P1-07](./units/P1-07.md) | Create walrus-sqlite crate with schema | P1-01, P1-02, P1-05 |
| [P1-08](./units/P1-08.md) | Implement SqliteMemory CRUD + FTS | P1-07 |
| [P1-09](./units/P1-09.md) | Implement SqliteMemory recall + compile_relevant | P1-08 |

## Dependency Graph

```text
P1-01 ─→ P1-02 ─→ P1-07 ─→ P1-08 ─→ P1-09
P1-05 ──────────↗
P1-03  (independent)
P1-04  (independent)
P1-06  (independent)
```

P1-03, P1-04, P1-05, P1-06 have no dependencies on each other and can be done in
any order. P1-07 through P1-09 are sequential.

## Workspace Changes

Add to `[workspace.dependencies]`:

```toml
rusqlite = { version = "0.34", features = ["bundled"] }
```

Add `crates/sqlite` to workspace members.

## Completion Checklist

- [ ] All 9 units complete
- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] `docs/src/design.md` updated to reflect actual implementations
- [ ] No unresolved design questions remain
