# crabtalk-berm

Crabtalk's side of [berm](https://github.com/crabtalk/berm): the harness hook and every system
harness Crabtalk serves — `crabtalk.fs`, `crabtalk.exec`, `crabtalk.http.fetch`,
one runtime capability per operation, and `berm.call`, which resolves the name
one harness reaches another by. berm serves none of its own.

berm itself has no crabtalk crate in its dependency list and cannot grow one
without `src/lib.rs` here moving — which is what makes "berm is embeddable
without crabtalk" compiler-checked rather than promised.

## License

Apache-2.0
