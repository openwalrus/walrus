# Phase 3: Gateway

Implement walrus-gateway — the application shell composing Runtime, channels,
sessions, authentication, and cron scheduling.

## Unit Index

| Unit | Title | Depends On |
|------|-------|------------|
| [P3-01](./units/P3-01.md) | Create walrus-gateway app skeleton | Phase 2 |
| [P3-02](./units/P3-02.md) | Walrus Protocol crate (app/protocol) | — |
| [P3-03](./units/P3-03.md) | Session management | P3-01 |
| [P3-04](./units/P3-04.md) | Authenticator trait and ApiKeyAuthenticator | P3-03 |
| [P3-05](./units/P3-05.md) | WebSocket server (axum + handler loop) | P3-01, P3-02, P3-03, P3-04 |
| [P3-06](./units/P3-06.md) | Channel routing | P3-03 |
| [P3-07](./units/P3-07.md) | Cron scheduler | P3-01 |
| [P3-08](./units/P3-08.md) | Gateway binary entry point | P3-05, P3-06, P3-07 |

## Dependency Graph

```text
P3-02 (independent) ──────→ P3-05 ─→ P3-08
P3-01 ─→ P3-03 ─→ P3-04 ──↗   ↗
     │        └──→ P3-06 ─────↗
     └─→ P3-07 ──────────────↗
```

P3-01 and P3-02 can start in parallel. P3-03, P3-07 start after P3-01.

## Workspace Changes

Add to `[workspace.dependencies]`:

```toml
protocol = { path = "app/protocol", package = "walrus-protocol", version = "0.0.9" }
axum = "0.8"
uuid = { version = "1", features = ["v4"] }
cron = "0.15"
jsonwebtoken = "9"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

Update `tokio` workspace dep to include `signal` feature.

Add `app/gateway` and `app/protocol` to workspace members.

## Completion Checklist

- [x] All 8 units complete
- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes (178 tests)
- [x] WebSocket connects, authenticates, sends/receives messages
- [x] Channel routing maps events to correct agents
- [x] Cron scheduler fires jobs on schedule
- [x] `docs/src/design.md` updated

## Deviations

- `GatewayHook` defined in `bin/main.rs` rather than a separate `hook.rs`
  file — the hook is zero-sized and only used at the monomorphization site.
- `jsonwebtoken` dep not added (JWT auth deferred; API key auth sufficient).
- Streaming wraps non-streaming `agent_send` for now (full `provider.stream()`
  integration is a follow-up).
- `SkillTier::Workspace` used instead of `Standard` (which doesn't exist).
