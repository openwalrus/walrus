//! Tests for CLI argument parsing.

use clap::Parser;
use walrus_cli::{Cli, Command};

#[test]
fn cli_parse_chat() {
    let cli = Cli::parse_from(["walrus", "chat"]);
    assert!(matches!(cli.command, Command::Chat));
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
fn cli_parse_agent_list() {
    let cli = Cli::parse_from(["walrus", "agent", "list"]);
    assert!(matches!(cli.command, Command::Agent { .. }));
}

#[test]
fn cli_parse_config_set() {
    let cli = Cli::parse_from(["walrus", "config", "set", "model", "deepseek-chat"]);
    assert!(matches!(cli.command, Command::Config { .. }));
}

#[test]
fn cli_parse_attach_default_url() {
    let cli = Cli::parse_from(["walrus", "attach"]);
    match cli.command {
        Command::Attach { url, auth_token } => {
            assert_eq!(url, "ws://127.0.0.1:6688/ws");
            assert!(auth_token.is_none());
        }
        _ => panic!("expected Attach command"),
    }
}

#[test]
fn cli_parse_attach_custom_url() {
    let cli = Cli::parse_from(["walrus", "attach", "--url", "ws://remote:9999/ws"]);
    match cli.command {
        Command::Attach { url, .. } => assert_eq!(url, "ws://remote:9999/ws"),
        _ => panic!("expected Attach command"),
    }
}

#[test]
fn cli_parse_attach_auth_token() {
    let cli = Cli::parse_from(["walrus", "attach", "--auth-token", "my-secret"]);
    match cli.command {
        Command::Attach { auth_token, .. } => assert_eq!(auth_token.as_deref(), Some("my-secret")),
        _ => panic!("expected Attach command"),
    }
}
