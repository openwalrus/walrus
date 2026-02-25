# Phase 4: CLI & Client

Implement walrus-client (shared client library) and walrus-cli (independent
command-line application with direct and gateway modes).

## Unit Index

| Unit | Title | Depends On |
|------|-------|------------|
| [P4-01](./units/P4-01.md) | walrus-client crate + WalrusClient struct | Phase 3 (P3-02) |
| [P4-02](./units/P4-02.md) | Client connect, send, stream | P4-01 |
| [P4-03](./units/P4-03.md) | walrus-cli crate skeleton + clap subcommands | — |
| [P4-04](./units/P4-04.md) | CLI direct mode (embed Runtime locally) | P4-03 |
| [P4-05](./units/P4-05.md) | CLI interactive chat REPL with streaming | P4-04 |
| [P4-06](./units/P4-06.md) | CLI gateway mode (connect via walrus-client) | P4-02, P4-05 |
| [P4-07](./units/P4-07.md) | CLI management commands (agent, memory, config) | P4-04 |

## Dependency Graph

```text
P4-01 ─→ P4-02 ──────────→ P4-06
P4-03 ─→ P4-04 ─→ P4-05 ──↗
              └──→ P4-07
```

P4-01 and P4-03 can start in parallel.

## Workspace Changes

Add to `[workspace.dependencies]`:

```toml
client = { path = "app/client", package = "walrus-client", version = "0.0.9" }
clap = { version = "4", features = ["derive"] }
tokio-tungstenite = "0.26"
rustyline = "15"
crossterm = "0.29"
dirs = "6"
```

## Completion Checklist

- [ ] All 7 units complete
- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] `walrus chat` works in direct mode (local Runtime)
- [ ] `walrus --gateway ws://... chat` works in gateway mode
- [ ] `walrus send "hello"` sends a one-shot message
- [ ] `walrus agent list` shows registered agents
- [ ] `docs/src/design.md` updated
