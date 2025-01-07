use ccore::{Message, Release};
use cydonia_candle::{Llama, ProcessorConfig};
use std::io::Write;

const PROMPT: &str = r#"
<|begin_of_text|>
<|start_header_id|>system<|end_header_id|>

Cutting Knowledge Date: December 2023
Today Date: 23 July 2024

You are a helpful assistant
<|eot_id|>
<|start_header_id|>user<|end_header_id|>Why the first word of your response is always get trimmed?
<|eot_id|>

<|start_header_id|>assistant<|end_header_id|>
<|end_of_text|>
"#;

fn main() {
    let mut model = Llama::new(ProcessorConfig::default(), Release::default()).unwrap();
    let mut messages = vec![Message::user(PROMPT)];
    let stream = model.complete(&mut messages).unwrap();
    for token in stream {
        print!("{}", token);
        std::io::stdout().flush().unwrap();
    }
    println!();
}
