//! Tests for CLI argument parsing.

use clap::Parser;
use walrus_cli::{Cli, Command};

#[test]
fn cli_parse_chat() {
    let cli = Cli::parse_from(["walrus", "chat"]);
    assert!(matches!(cli.command, Command::Chat));
    assert!(cli.gateway.is_none());
}

#[test]
fn cli_parse_send() {
    let cli = Cli::parse_from(["walrus", "send", "hello world"]);
    match cli.command {
        Command::Send { content } => assert_eq!(content, "hello world"),
        _ => panic!("expected Send command"),
    }
}

#[test]
fn cli_parse_gateway_flag() {
    let cli = Cli::parse_from(["walrus", "--gateway", "ws://example.com/ws", "chat"]);
    assert_eq!(cli.gateway.as_deref(), Some("ws://example.com/ws"));
}

#[test]
fn cli_parse_model_flag() {
    let cli = Cli::parse_from(["walrus", "--model", "gpt-4", "chat"]);
    assert_eq!(cli.model.as_deref(), Some("gpt-4"));
}

#[test]
fn cli_parse_agent_flag() {
    let cli = Cli::parse_from(["walrus", "--agent", "helper", "send", "hi"]);
    assert_eq!(cli.agent.as_deref(), Some("helper"));
}

#[test]
fn cli_parse_config_flag() {
    let cli = Cli::parse_from(["walrus", "--config", "/tmp/test.toml", "chat"]);
    assert_eq!(cli.config.as_deref(), Some("/tmp/test.toml"));
}

#[test]
fn cli_parse_agent_list() {
    let cli = Cli::parse_from(["walrus", "agent", "list"]);
    assert!(matches!(cli.command, Command::Agent { .. }));
}

#[test]
fn cli_parse_config_set() {
    let cli = Cli::parse_from(["walrus", "config", "set", "model", "deepseek-chat"]);
    assert!(matches!(cli.command, Command::Config { .. }));
}
