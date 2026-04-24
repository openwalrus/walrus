# crabtalkd

The Crabtalk daemon binary.

Wraps `crabtalk` (the daemon library) with a minimal CLI: run the event loop
(what the service unit invokes), scaffold first-run config interactively,
hot-reload, stream events, and install/uninstall runtime plugins over the
socket. The daemon no longer manages its own service lifecycle — that's
[crabup](../crabup).

## Usage

```bash
crabtalkd setup              # one-time interactive LLM endpoint config
crabtalkd run                # run in the foreground (launchd/systemd invokes this)
crabtalkd reload             # hot-reload config over the socket
crabtalkd events             # stream agent/tool events
crabtalkd pull <plugin>      # install a runtime plugin into a running daemon
crabtalkd rm <plugin>        # uninstall a runtime plugin
```

Install/start as a service via `crabup daemon start`.

## Features

- `native-tls` (default) — OS TLS stack (SecureTransport on macOS, OpenSSL on Linux)
- `rustls` — pure-Rust TLS via rustls (for cross-compilation)

## License

MIT OR Apache-2.0
