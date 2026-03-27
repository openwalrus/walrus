# 0043 - Command

- Feature Name: Command System
- Start Date: 2026-02-15
- Discussion: [#43](https://github.com/crabtalk/crabtalk/issues/43)
- Crates: command

## Summary

A shared crate for system service management. Every crabtalk binary that runs
as a service uses the command crate for lifecycle (install/start/stop/logs) and
runtime bootstrap.

## Motivation

Crabtalk runs multiple binaries as system services — the daemon, search engine,
and potentially more. Each needs the same service management: install as a
launchd/systemd/schtasks service, view logs, start and stop. Without a shared
crate, every binary reimplements this.

## Design

### Service trait

```rust
pub trait Service {
    fn name(&self) -> &str;        // "search"
    fn description(&self) -> &str; // human readable
    fn label(&self) -> &str;       // "ai.crabtalk.search"
}
```

Implementors provide metadata. The trait provides default `start`, `stop`, and
`logs` methods. `start` renders a platform-specific template, installs it, and
launches the service. `stop` uninstalls and removes the port file. `logs` tails
`~/.crabtalk/logs/{name}.log`.

Service is a trait rather than a struct because `McpService` extends it — MCP
services need the same lifecycle management plus an HTTP router.

### Platform support

Service templates are platform-specific static strings with placeholder
substitution:

- **macOS** — launchd plist (`~/Library/LaunchAgents/`)
- **Linux** — systemd user unit
- **Windows** — schtasks with XML task definition

### MCP service

Services that expose an HTTP API extend `McpService`:

```rust
pub trait McpService: Service {
    fn router(&self) -> axum::Router;
}
```

`run_mcp` binds a TCP listener on `127.0.0.1:0`, writes the port to
`~/.crabtalk/run/{name}.port`, and serves the router.

### Auto-discovery: port files, not subprocesses

The daemon does not manage MCP servers as subprocesses. Other projects spawn MCP
servers as child processes — if the child hangs or crashes, it can take the
daemon with it. A broken subprocess can trigger broken daemon behavior: zombie
processes, leaked file descriptors, blocked event loops.

Crabtalk's approach: MCP servers are independent system services. Each writes a
port file (`~/.crabtalk/run/{name}.port`) on startup. The daemon scans this
directory at startup and discovers services automatically. No subprocess
management, no shared fate.

The contract is simple: write a port file, the daemon finds you. Crash? The
daemon doesn't care — it was never your parent process. Restart? New port file,
the daemon picks it up on next reload. This is the manifesto applied to tool
servers: they connect as clients, they crash alone.

### Entry point

The `run()` function handles tracing init and tokio bootstrap for all binaries.

## Alternatives

**Shell scripts for service management.** Works on Unix, breaks on Windows,
drifts across binaries. A shared Rust crate is portable and stays consistent.

## Unresolved Questions

- Should the Service trait support health checks?
