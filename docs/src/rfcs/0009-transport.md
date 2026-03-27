# 0009 - Transport

- Feature Name: UDS and TCP Transport Layers
- Start Date: 2026-03-27
- Discussion: [#9](https://github.com/crabtalk/crabtalk/issues/9)
- Crates: transport, core

## Summary

A transport layer providing Unix domain socket (UDS) and TCP connectivity
between clients and the crabtalk daemon, built on a shared length-prefixed
protobuf codec defined in `core`.

## Motivation

The daemon needs to accept connections from local CLI clients and remote clients
(Telegram, web gateways). UDS is the natural choice for same-machine
communication — no port management, filesystem-based access control. TCP is
required for remote access and cross-platform support (Windows has no UDS).

Both transports share identical framing and message types. The codec and message
definitions belong in `core` so that any transport can use them without
depending on each other. The `transport` crate provides the concrete connection
machinery.

## Design

### Codec (`core::protocol::codec`)

Wire format: `[u32 BE length][protobuf payload]`. The length prefix counts
payload bytes only, excluding the 4-byte header itself.

Two generic async functions operate over any `AsyncRead`/`AsyncWrite`:

- `write_message<W, T: Message>(writer, msg)` — encode, length-prefix, flush.
- `read_message<R, T: Message + Default>(reader)` — read length, read payload,
  decode.

Maximum frame size is 16 MiB. Frames exceeding this limit produce a
`FrameError::TooLarge`. EOF during the length read produces
`FrameError::ConnectionClosed` (clean disconnect, not an error).

### Server accept loop

Both UDS and TCP servers share the same pattern:

```
accept_loop(listener, on_message, shutdown)
```

- `listener` — `UnixListener` or `TcpListener`.
- `on_message: Fn(ClientMessage, Sender<ServerMessage>)` — called for
  each decoded client message. The sender is per-connection; the callback can
  send multiple `ServerMessage`s (streaming responses) or exactly one
  (request-response). The channel is unbounded because messages are small and
  flow-controlled by the protocol — the agent produces responses at LLM speed,
  far slower than socket drain speed.
- `shutdown` — `oneshot::Receiver<()>` for graceful stop.

Each accepted connection spawns two tasks: a read loop that decodes
`ClientMessage`s and calls `on_message`, and a send task that drains the
`UnboundedSender` and writes `ServerMessage`s back. When the read loop ends
(EOF or error), the sender is dropped, which terminates the send task.

### TCP specifics

- Default port: `6688`. If the port is in use, bind fails — another daemon may
  already be running.
- `TCP_NODELAY` is set on all connections (low-latency interactive protocol).
- `bind()` returns a `std::net::TcpListener` (non-blocking).

### UDS specifics

- Unix-only (`#[cfg(unix)]`).
- Socket path is caller-provided (typically `~/.crabtalk/daemon.sock`).
- No port management or collision handling — the filesystem path is the
  identity.

### Client trait (`core::protocol::api::Client`)

Two required transport primitives:

- `request(ClientMessage) -> Result<ServerMessage>` — single round-trip.
- `request_stream(ClientMessage) -> Stream<Item = Result<ServerMessage>>` —
  send one message, read responses until the stream ends.

Both UDS `Connection` and TCP `TcpConnection` implement `Client` identically:
split the socket into owned read/write halves, write via codec, read via codec.
The `request_stream` implementation reads indefinitely; typed provided methods
on `Client` (e.g., `stream()`) handle sentinel detection (`StreamEnd`).

Connections are not `Clone` — one connection per session. The client struct
(`CrabtalkClient` / `TcpClient`) holds config and produces connections on
demand.

## Alternatives

**tokio-util `LengthDelimitedCodec`.** Would save the manual length-prefix
code but adds a dependency for ~50 lines of straightforward framing. The
hand-rolled codec is simpler to audit and has no extra allocations.

**gRPC / tonic.** Full RPC framework with HTTP/2 transport. Heavyweight for a
local daemon protocol. The current design is simpler: raw protobuf over a
length-prefixed stream, no HTTP layer, no service definitions beyond the
`Server` trait.

**Shared generic transport trait.** UDS and TCP accept loops are nearly
identical but kept as separate modules. A generic `Transport` trait would save
~20 lines of duplication but add an abstraction with exactly two implementors.
Not worth it.

## Unresolved Questions

- Should the transport support TLS for TCP connections in non-localhost
  deployments?
- Should there be a connection timeout or keepalive at the transport level, or
  is the protocol-level `Ping`/`Pong` sufficient?
