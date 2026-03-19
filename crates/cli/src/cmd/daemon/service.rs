//! `crabtalk daemon start/stop` — system service management.

use crate::cmd::attach::setup_provider;
use anyhow::Result;
use wcore::{paths::CONFIG_DIR, service::ServiceParams};

#[cfg(target_os = "macos")]
const LAUNCHD_TEMPLATE: &str = include_str!("launchd.plist");
#[cfg(target_os = "linux")]
const SYSTEMD_TEMPLATE: &str = include_str!("systemd.service");

/// Check if providers are configured; scaffold config and prompt if needed.
fn ensure_providers() -> Result<()> {
    let config_path = CONFIG_DIR.join("crab.toml");
    if !config_path.exists() {
        ::daemon::config::scaffold_config_dir(&CONFIG_DIR)?;
    }

    let config = ::daemon::DaemonConfig::load(&config_path)?;
    if config.provider.is_empty() {
        setup_provider(&config_path)?;
    }
    Ok(())
}

fn daemon_params() -> Result<ServiceParams<'static>> {
    // Leak the binary path so we can return 'static refs.
    // Only called once per process invocation.
    let binary = Box::leak(std::env::current_exe()?.into_boxed_path());
    let socket = Box::leak(wcore::paths::SOCKET_PATH.clone().into_boxed_path());
    let config_path = Box::leak(CONFIG_DIR.join("crab.toml").into_boxed_path());
    Ok(ServiceParams {
        label: "ai.crabtalk.crabtalk",
        description: "Crabtalk Daemon",
        subcommand: "daemon",
        log_name: "daemon",
        binary,
        socket,
        config_path,
    })
}

#[cfg(target_os = "macos")]
pub fn install() -> Result<()> {
    ensure_providers()?;
    let params = daemon_params()?;
    wcore::service::install(LAUNCHD_TEMPLATE, &params)
}

#[cfg(target_os = "macos")]
pub fn uninstall() -> Result<()> {
    let params = daemon_params()?;
    wcore::service::uninstall(&params)
}

#[cfg(target_os = "linux")]
pub fn install() -> Result<()> {
    ensure_providers()?;
    let params = daemon_params()?;
    wcore::service::install(SYSTEMD_TEMPLATE, &params)
}

#[cfg(target_os = "linux")]
pub fn uninstall() -> Result<()> {
    let params = daemon_params()?;
    wcore::service::uninstall(&params)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn install() -> Result<()> {
    anyhow::bail!("daemon start is only supported on macOS and Linux")
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn uninstall() -> Result<()> {
    anyhow::bail!("daemon stop is only supported on macOS and Linux")
}
