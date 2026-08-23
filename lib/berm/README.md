# berm-crabtalk

The guest half of the `crabtalk` namespace: `fs`, `exec`, `http`, and one door
per runtime operation — `peers`, `sessions`, `skills`. A harness links this to
reach a Crabtalk daemon; [`crabtalk-berm`](../../crates/berm) serves the other
end of every name in it.

```rust
let agents = protocol::peers()?;              // the other agents
let body = protocol::skill("review")?;        // one skill's instructions
let bytes = fs::read("src/lib.rs")?;          // inside the granted root
```

[`berm-lang`](https://crates.io/crates/berm-lang) owns the ABI and declares no
namespace of its own, because what a harness can reach is a decision about a
host and berm has no host. This is that decision, for one.

Drift between the two halves is caught rather than prevented: a renamed call
hashes to a number nothing is registered for, and it is loud on the first call.

## License

Apache-2.0
