# Cydonia

Cydonia is a library based on [candle][candle] for developing modern AI applications in rust.

```rust
use cydonia::Model;

fn main() {
    let model = Model::new("llama3.2-1b");
    let response = model.invoke("Hello, world!");
    println!("{}", response);
}
```

## LICENSE

[GPL-3.0](LICENSE)

<!-- links -->

[candle]: https://github.com/huggingface/candle
[ollama]: https://github.com/ollama/ollama
