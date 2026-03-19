//! `crabtalk-gateway` binary — manages gateway services.

use clap::{Parser, Subcommand};
use crabtalk_gateway::{GatewayConfig, config::TelegramConfig, service};
use dialoguer::{Password, theme::ColorfulTheme};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "crabtalk-gateway", about = "Crabtalk gateway manager")]
struct App {
    #[command(subcommand)]
    command: GatewayCommand,
}

#[derive(Subcommand)]
enum GatewayCommand {
    /// Manage the Telegram gateway.
    Telegram {
        #[command(subcommand)]
        action: TelegramAction,
    },
}

#[derive(Subcommand)]
enum TelegramAction {
    /// Install and start the Telegram gateway as a system service.
    Start,
    /// Stop and uninstall the Telegram gateway system service.
    Stop,
    /// Run the Telegram gateway directly (used by launchd/systemd).
    Run {
        /// Daemon UDS socket path.
        #[arg(long)]
        daemon: String,
        /// Path to gateway config file.
        #[arg(long)]
        config: PathBuf,
    },
}

fn default_config_path() -> PathBuf {
    wcore::paths::CONFIG_DIR.join("gateway.toml")
}

fn resolve_binary() -> anyhow::Result<PathBuf> {
    Ok(std::env::current_exe()?)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let level = std::env::var("RUST_LOG")
        .ok()
        .map(
            |v| match v.rsplit('=').next().unwrap_or(&v).to_lowercase().as_str() {
                "trace" => tracing::Level::TRACE,
                "debug" => tracing::Level::DEBUG,
                "info" => tracing::Level::INFO,
                "error" => tracing::Level::ERROR,
                _ => tracing::Level::WARN,
            },
        )
        .unwrap_or(tracing::Level::WARN);
    tracing_subscriber::fmt().with_max_level(level).init();

    let app = App::parse();
    match app.command {
        GatewayCommand::Telegram { action } => match action {
            TelegramAction::Start => telegram_start().await,
            TelegramAction::Stop => telegram_stop(),
            TelegramAction::Run { daemon, config } => telegram_run(&daemon, &config).await,
        },
    }
}

async fn telegram_start() -> anyhow::Result<()> {
    let config_path = default_config_path();

    // Load or create config, prompting for token if missing.
    let mut config = if config_path.exists() {
        GatewayConfig::load(&config_path)?
    } else {
        GatewayConfig::default()
    };

    if config.telegram.as_ref().is_none_or(|t| t.token.is_empty()) {
        let token = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Telegram bot token (from @BotFather)")
            .interact()?;
        if token.is_empty() {
            anyhow::bail!("token cannot be empty");
        }
        config.telegram = Some(TelegramConfig {
            token,
            allowed_users: vec![],
        });
        config.save(&config_path)?;
        println!("saved config to {}", config_path.display());
    }

    let binary = resolve_binary()?;
    let socket = wcore::paths::SOCKET_PATH.clone();

    let params = service::ServiceParams {
        label: "ai.crabtalk.gateway-telegram",
        description: "Telegram",
        subcommand: "telegram",
        log_name: "gateway-telegram",
        binary: &binary,
        socket: &socket,
        config_path: &config_path,
    };
    service::install_gateway(&params)
}

fn telegram_stop() -> anyhow::Result<()> {
    let socket = wcore::paths::SOCKET_PATH.clone();
    let config_path = default_config_path();
    let binary = resolve_binary()?;

    let params = service::ServiceParams {
        label: "ai.crabtalk.gateway-telegram",
        description: "Telegram",
        subcommand: "telegram",
        log_name: "gateway-telegram",
        binary: &binary,
        socket: &socket,
        config_path: &config_path,
    };
    service::uninstall(&params)
}

async fn telegram_run(daemon_socket: &str, config_path: &std::path::Path) -> anyhow::Result<()> {
    let config = if config_path.exists() {
        GatewayConfig::load(config_path)?
    } else {
        GatewayConfig::default()
    };
    crabtalk_gateway::telegram::serve::run(daemon_socket, &config).await
}
