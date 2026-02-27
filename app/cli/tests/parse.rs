//! Tests for CLI argument parsing.

use clap::Parser;
use walrus_cli::{Cli, Command};

#[test]
fn cli_parse_chat() {
    let cli = Cli::parse_from(["walrus", "chat"]);
    assert!(matches!(cli.command, Command::Chat(..)));
}

#[test]
fn cli_parse_send() {
    let cli = Cli::parse_from(["walrus", "send", "hello world"]);
    match cli.command {
        Command::Send(cmd) => assert_eq!(cmd.content, "hello world"),
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
    assert!(matches!(cli.command, Command::Agent(..)));
}

#[test]
fn cli_parse_config_set() {
    let cli = Cli::parse_from(["walrus", "config", "set", "model", "deepseek-chat"]);
    assert!(matches!(cli.command, Command::Config(..)));
}

#[test]
fn cli_parse_serve_default() {
    let cli = Cli::parse_from(["walrus", "serve"]);
    match cli.command {
        Command::Serve(cmd) => assert!(cmd.socket.is_none()),
        _ => panic!("expected Serve command"),
    }
}

#[test]
fn cli_parse_serve_custom_socket() {
    let cli = Cli::parse_from(["walrus", "serve", "--socket", "/tmp/walrus.sock"]);
    match cli.command {
        Command::Serve(cmd) => {
            assert_eq!(
                cmd.socket.unwrap(),
                std::path::PathBuf::from("/tmp/walrus.sock")
            );
        }
        _ => panic!("expected Serve command"),
    }
}

#[test]
fn cli_parse_attach_default_socket() {
    let cli = Cli::parse_from(["walrus", "attach"]);
    match cli.command {
        Command::Attach(cmd) => {
            assert!(cmd.socket.is_none());
        }
        _ => panic!("expected Attach command"),
    }
}

#[test]
fn cli_parse_attach_custom_socket() {
    let cli = Cli::parse_from(["walrus", "attach", "--socket", "/tmp/test.sock"]);
    match cli.command {
        Command::Attach(cmd) => {
            assert_eq!(
                cmd.socket.unwrap(),
                std::path::PathBuf::from("/tmp/test.sock")
            );
        }
        _ => panic!("expected Attach command"),
    }
}

