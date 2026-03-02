//! Shared helpers for runtime examples.

#![allow(dead_code)]

use model::ProviderManager;
use walrus_runtime::{Hook, Memory, prelude::*};
use wcore::AgentEvent;

/// Example hook wiring ProviderManager as the model provider.
pub struct ExampleHook;

impl Hook for ExampleHook {
    type Model = ProviderManager;
    type Memory = InMemory;
}

/// Initialize tracing with env-filter support.
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
}

/// Load DEEPSEEK_API_KEY from .env file, falling back to environment.
pub fn load_api_key() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set")
}

/// Build a default Runtime with DeepSeek provider and InMemory.
pub fn build_runtime() -> Runtime<ExampleHook> {
    let key = load_api_key();
    let config = model::ProviderConfig {
        model: "deepseek-chat".into(),
        api_key: Some(key.into()),
        base_url: None,
        loader: None,
        quantization: None,
        chat_template: None,
    };
    let provider =
        model::deepseek::DeepSeek::new(model::Client::new(), &config.api_key.as_ref().unwrap())
            .expect("failed to create provider");
    let manager = ProviderManager::single(config, model::Provider::DeepSeek(provider));
    Runtime::new(Request::default(), manager, InMemory::new())
}

/// Simple REPL loop: read lines from stdin, stream to agent.
pub async fn repl<H: Hook + 'static>(runtime: &Runtime<H>, agent: &str) {
    use futures_util::StreamExt;
    use std::io::{BufRead, Write};

    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        if std::io::stdin().lock().read_line(&mut input).unwrap() == 0 {
            break;
        }
        let input = input.trim();
        if input.is_empty() || input == "exit" || input == "quit" {
            break;
        }
        let mut stream = std::pin::pin!(runtime.stream_to(agent, Message::user(input)));
        while let Some(event) = stream.next().await {
            if let AgentEvent::TextDelta(text) = &event {
                print!("{text}");
                std::io::stdout().flush().ok();
            }
        }
        println!();
    }
}

/// REPL loop that prints memory entries after each exchange.
pub async fn repl_with_memory<H: Hook + 'static>(runtime: &Runtime<H>, agent: &str) {
    use futures_util::StreamExt;
    use std::io::{BufRead, Write};

    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        if std::io::stdin().lock().read_line(&mut input).unwrap() == 0 {
            break;
        }
        let input = input.trim();
        if input.is_empty() || input == "exit" || input == "quit" {
            break;
        }
        {
            let mut stream = std::pin::pin!(runtime.stream_to(agent, Message::user(input)));
            while let Some(event) = stream.next().await {
                if let AgentEvent::TextDelta(text) = &event {
                    print!("{text}");
                    std::io::stdout().flush().ok();
                }
            }
            println!();
        }

        // Print current memory state (stream dropped, borrow released).
        let entries = runtime.memory().entries();
        if entries.is_empty() {
            println!("[Memory: empty]");
        } else {
            println!("[Memory: {} entries]", entries.len());
            for (key, value) in &entries {
                let display = if value.len() > 60 {
                    let end = value
                        .char_indices()
                        .take_while(|&(i, _)| i <= 57)
                        .last()
                        .map(|(i, c)| i + c.len_utf8())
                        .unwrap_or(0);
                    format!("{}...", &value[..end])
                } else {
                    value.clone()
                };
                println!("  {key} = {display}");
            }
        }
    }
}
