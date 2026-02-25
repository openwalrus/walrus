# Phase 3: Gateway

Implement walrus-gateway — the application shell composing Runtime, channels,
sessions, authentication, and cron scheduling.

## Unit Index

| Unit | Title | Depends On |
|------|-------|------------|
| [P3-01](./units/P3-01.md) | Create walrus-gateway crate skeleton | Phase 2 |
| [P3-02](./units/P3-02.md) | Walrus Protocol types (ClientMessage, ServerMessage) | P3-01 |
| [P3-03](./units/P3-03.md) | Session management | P3-01 |
| [P3-04](./units/P3-04.md) | Authenticator trait and ApiKeyAuthenticator | P3-03 |
| [P3-05](./units/P3-05.md) | WebSocket server (axum + handler loop) | P3-02, P3-03, P3-04 |
| [P3-06](./units/P3-06.md) | Channel routing | P3-03 |
| [P3-07](./units/P3-07.md) | Cron scheduler | P3-01 |
| [P3-08](./units/P3-08.md) | Binary entry point | P3-05, P3-06, P3-07 |

## Dependency Graph

```text
P3-01 ─→ P3-02 ──────→ P3-05 ─→ P3-08
     ├─→ P3-03 ─→ P3-04 ↗   ↗
     │        └──→ P3-06 ───↗
     └─→ P3-07 ────────────↗
```

P3-02, P3-03, P3-07 can start in parallel after P3-01.

## Workspace Changes

Add to `[workspace.dependencies]`:

```toml
axum = "0.8"
tokio-tungstenite = "0.26"
uuid = { version = "1", features = ["v4"] }
cron = "0.15"
jsonwebtoken = "9"
```

Add `crates/gateway` to workspace members.

## Completion Checklist

- [ ] All 8 units complete
- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] WebSocket connects, authenticates, sends/receives messages
- [ ] Channel routing maps events to correct agents
- [ ] Cron scheduler fires jobs on schedule
- [ ] `docs/src/design.md` updated
