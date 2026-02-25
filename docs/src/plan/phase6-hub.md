# Phase 6: Hub

GitHub-based resource registry for sharing workspaces, skills, agents, and
MCP configs. Modeled after Homebrew: a central GitHub repo with TOML manifests,
local clone for search, shell git for all operations.

## Unit Index

| Unit | Title | Depends On |
|------|-------|------------|
| [P6-01](./units/P6-01.md) | Hub manifest types and parsing | P5-01 |
| [P6-02](./units/P6-02.md) | Hub repo management (clone, pull, local index) | P6-01 |
| [P6-03](./units/P6-03.md) | Hub search and info | P6-02 |
| [P6-04](./units/P6-04.md) | Hub install and remove | P6-02 |
| [P6-05](./units/P6-05.md) | Hub update and lockfile management | P6-04 |
| [P6-06](./units/P6-06.md) | Hub CLI wiring (`walrus hub` subcommand) | P6-03, P6-04, P6-05 |

## Dependency Graph

```text
P6-01 ─→ P6-02 ─→ P6-03 ──────→ P6-06
              └──→ P6-04 ─→ P6-05 ─↗
```

P6-03 and P6-04 can proceed in parallel after P6-02.

## Workspace Changes

New crate: `app/hub` added to workspace members.

Add to `[workspace.dependencies]`:

```toml
hub = { path = "app/hub", package = "walrus-hub", version = "0.0.9" }
tempfile = "3"
```

`dirs` already in workspace from Phase 4. No `git2` — shell out to
system `git` (DD#35).

## Completion Checklist

- [ ] All 6 units complete
- [ ] `walrus hub update` clones/pulls the default hub repo
- [ ] `walrus hub search <query>` finds matching resources
- [ ] `walrus hub install <name>` installs into workspace
- [ ] `walrus hub remove <name>` removes installed resource
- [ ] `walrus hub info <name>` shows manifest details
- [ ] `hub.lock` tracks installed resources
- [ ] Multiple hub sources supported
- [ ] `cargo check --workspace` and `cargo test --workspace` pass
- [ ] `docs/src/design.md` updated
