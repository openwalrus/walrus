use ccore::{Message, Release};
use cydonia_candle::{Llama, ProcessorConfig};
use std::io::Write;

fn main() {
    let mut model = Llama::new(ProcessorConfig::default(), Release::default()).unwrap();

    let mut last = None;
    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();

        // Read input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if input.ends_with('\n') {
            input.pop();
            if input.ends_with('\r') {
                input.pop();
            }
        }

        // Generate response
        let mut response = String::new();
        let message = Message::user(input);
        let mut stream = model.complete(&[message], last).unwrap();
        while let Some(token) = stream.next() {
            response.push_str(&token);

            print!("{}", token);
            std::io::stdout().flush().unwrap();
        }
        println!();

        last = Some(Message::assistant(response));
    }
}
