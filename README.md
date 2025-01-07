# Cydonia

Cydonia is a library based on [candle][candle] for developing modern AI applications in rust.

```rust
use cydonia::Model;

fn main() {
    let model = Model::new("gemma2").tag("latest");
    let response = model.invoke("Hello, world!");
    println!("{}", response);
}
```

We support quantized models only derived from `gemma` and `llama` family.

## TODOs

- [x] Support chat interface ( history prompts )
- [ ] Function encoder for llama3 tools (static)
- [ ] Cydonia as service
  - [ ] RPC support for llama3 tools (remote)
  - [ ] GraphQL support for llama3 tools (remote)
- [ ] RAG support
- [ ] Agent interface
- [ ] Multi-agent support (single-node)
- [ ] An application based on the tools
- [ ] p2p for the decentralized cydonia network (multi-node)
- [ ] Test gpu

<!-- links -->

[candle]: https://github.com/huggingface/candle
