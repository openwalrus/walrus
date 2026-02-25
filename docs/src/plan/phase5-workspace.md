# Phase 5: Workspace + OpenAPI

Introduce the Workspace concept (project directories for walrus configurations)
and REST/OpenAPI endpoints alongside the existing WebSocket server.

The marketing agent from earlier planning moves to `examples/marketing/` as
a reference workspace validating the format.

## Unit Index

| Unit | Title | Depends On |
|------|-------|------------|
| [P5-01](./units/P5-01.md) | Workspace config schema and types | P3-08 |
| [P5-02](./units/P5-02.md) | `walrus init` command | P5-01, P4-03, P4-07 |
| [P5-03](./units/P5-03.md) | Workspace-aware GatewayConfig loading | P5-01 |
| [P5-04](./units/P5-04.md) | OpenAPI schema derives on protocol types | P3-02 |
| [P5-05](./units/P5-05.md) | REST endpoints (send, stream, agents, health) | P5-04, P3-05, P3-08 |
| [P5-06](./units/P5-06.md) | Swagger UI and OpenAPI spec endpoint | P5-05 |
| [P5-07](./units/P5-07.md) | Marketing agent example workspace | P5-02 |
| [P5-08](./units/P5-08.md) | `walrus attach` command | P5-01, P4-06 |

## Dependency Graph

```text
P5-01 ─→ P5-02 ──────────→ P5-07
     ├─→ P5-03
     └─→ P5-08
P5-04 ─→ P5-05 ─→ P5-06
```

P5-01/P5-04 can start in parallel. P5-02/P5-03/P5-08 start after P5-01.
P5-05/P5-06 are independent of the workspace units.

## Workspace Changes

Add to `[workspace.dependencies]`:

```toml
utoipa = { version = "5", features = ["axum_extras"] }
utoipa-axum = "0.2"
utoipa-swagger-ui = { version = "9", features = ["axum"] }
```

## Completion Checklist

- [ ] All 8 units complete
- [ ] `walrus init my-project` creates correct directory structure
- [ ] Gateway loads config from workspace directory
- [ ] REST endpoints return correct responses
- [ ] OpenAPI spec generated and served at `/api-docs/openapi.json`
- [ ] Swagger UI accessible at `/swagger-ui`
- [ ] `examples/marketing/` is a valid workspace
- [ ] `walrus attach` connects to running gateway from workspace
- [ ] `cargo check --workspace` and `cargo test --workspace` pass
- [ ] `docs/src/design.md` updated
