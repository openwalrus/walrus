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

## Special Thanks

- [candle][candle]
- [ollama][ollama]

<!-- links -->

[candle]: https://github.com/huggingface/candle
[ollama]: https://github.com/ollama/ollama
