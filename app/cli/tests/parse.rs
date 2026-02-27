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
fn cli_parse_socket_flag() {
    let cli = Cli::parse_from(["walrus", "--socket", "/tmp/walrus.sock", "chat"]);
    assert_eq!(
        cli.socket.unwrap(),
        std::path::PathBuf::from("/tmp/walrus.sock")
    );
}
