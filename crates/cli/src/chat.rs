//! Chat command

use super::Config;
use anyhow::Result;
use chrono::Utc;
use clap::{Args, ValueEnum};
use cydonia::{Agent, Chat, Client, Message, Provider, Runtime, Tool};
use futures_util::StreamExt;
use schemars::JsonSchema;
use serde::Deserialize;
use std::{
    fmt::{Display, Formatter},
    io::{BufRead, Write},
};

/// Chat command arguments
#[derive(Debug, Args)]
pub struct ChatCmd {
    /// The model provider to use
    #[arg(short, long, default_value = "deepseek")]
    pub model: Model,

    /// The agent to use for the chat
    #[arg(short, long)]
    pub agent: Option<AgentKind>,

    /// Whether to enable thinking
    #[arg(short, long)]
    pub think: bool,

    /// The message to send (if empty, starts interactive mode)
    pub message: Option<String>,
}

/// Available agent types
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AgentKind {
    /// Anto - basic agent with time tool for testing
    Anto,
}

/// Parameters for the get_time tool
#[allow(dead_code)]
#[derive(JsonSchema, Deserialize)]
struct GetTimeParams {
    /// If returns UNIX timestamp instead
    timestamp: bool,
}

fn get_time_tool() -> Tool {
    Tool {
        name: "get_time".into(),
        description: "Gets the current UTC time in ISO 8601 format.".into(),
        parameters: schemars::schema_for!(GetTimeParams),
        strict: true,
    }
}

impl ChatCmd {
    /// Run the chat command
    pub async fn run(&self, stream: bool) -> Result<()> {
        let mut config = Config::load()?;
        let key = config
            .key
            .get(&self.model.to_string())
            .ok_or_else(|| anyhow::anyhow!("missing {:?} API key in config", self.model))?;

        let provider = Provider::new(&config.config.model, Client::new(), key)?;

        // override the think flag in the config
        config.config.think = self.think;
        let general = config.config().clone();

        // Build runtime with agent + tools
        let mut runtime = Runtime::new(general, provider);

        let agent = match self.agent {
            Some(AgentKind::Anto) => {
                runtime.register(get_time_tool(), |args| async move {
                    let args: GetTimeParams =
                        serde_json::from_str(&args).unwrap_or(GetTimeParams { timestamp: false });
                    if args.timestamp {
                        Utc::now().timestamp().to_string()
                    } else {
                        Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
                    }
                });

                Agent::new("anto")
                    .system_prompt(
                        "You are Anto, a helpful assistant. You can get the current time.",
                    )
                    .tool("get_time")
            }
            None => Agent::new("assistant").system_prompt("You are a helpful assistant."),
        };

        runtime.add_agent(agent.clone());
        let mut chat = runtime.chat(&agent.name)?;
        self.run_chat(&runtime, &mut chat, stream).await
    }

    async fn run_chat(&self, runtime: &Runtime, chat: &mut Chat, stream: bool) -> Result<()> {
        if let Some(msg) = &self.message {
            Self::send(runtime, chat, Message::user(msg), stream).await?;
        } else {
            let stdin = std::io::stdin();
            let mut stdout = std::io::stdout();
            loop {
                print!("> ");
                stdout.flush()?;

                let mut input = String::new();
                if stdin.lock().read_line(&mut input)? == 0 {
                    break;
                }

                let input = input.trim();
                if input.is_empty() {
                    continue;
                }
                if input == "/quit" || input == "/exit" {
                    break;
                }

                Self::send(runtime, chat, Message::user(input), stream).await?;
            }
        }

        Ok(())
    }

    async fn send(
        runtime: &Runtime,
        chat: &mut Chat,
        message: Message,
        stream: bool,
    ) -> Result<()> {
        if stream {
            let mut reasoning = false;
            let mut stream = std::pin::pin!(runtime.stream(chat, message));
            while let Some(Ok(chunk)) = stream.next().await {
                if let Some(content) = chunk.content() {
                    if reasoning {
                        println!("\n\n\nCONTENT");
                        reasoning = false;
                    }
                    print!("{content}");
                }

                if let Some(reasoning_content) = chunk.reasoning_content() {
                    if !reasoning {
                        println!("REASONING");
                        reasoning = true;
                    }
                    print!("{reasoning_content}");
                }
            }
            println!();
        } else {
            let response = runtime.send(chat, message).await?;
            if let Some(reasoning_content) = response.reasoning() {
                println!("REASONING\n{reasoning_content}");
            }

            if let Some(content) = response.content() {
                println!("\n\nCONTENT\n{content}");
            }
        }
        Ok(())
    }
}

/// Available model providers
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Model {
    /// DeepSeek model
    #[default]
    Deepseek,
}

impl Display for Model {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::Deepseek => write!(f, "deepseek"),
        }
    }
}
