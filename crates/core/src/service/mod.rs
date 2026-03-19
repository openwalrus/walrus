//! Shared system service management (launchd/systemd).

use crate::paths::{CONFIG_DIR, LOGS_DIR};
use std::path::Path;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

/// Parameters for rendering a service template.
pub struct ServiceParams<'a> {
    pub label: &'a str,
    pub description: &'a str,
    pub subcommand: &'a str,
    pub log_name: &'a str,
    pub binary: &'a Path,
    pub socket: &'a Path,
    pub config_path: &'a Path,
}

/// Render a template by replacing placeholder tokens with values from `params`.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn render_template(template: &str, params: &ServiceParams<'_>) -> String {
    let path = std::env::var("PATH").unwrap_or_default();
    template
        .replace("{label}", params.label)
        .replace("{description}", params.description)
        .replace("{subcommand}", params.subcommand)
        .replace("{log_name}", params.log_name)
        .replace("{binary}", &params.binary.display().to_string())
        .replace("{socket}", &params.socket.display().to_string())
        .replace("{config_path}", &params.config_path.display().to_string())
        .replace("{logs_dir}", &LOGS_DIR.display().to_string())
        .replace("{config_dir}", &CONFIG_DIR.display().to_string())
        .replace("{path}", &path)
}

#[cfg(target_os = "macos")]
pub use macos::{install, uninstall};

#[cfg(target_os = "linux")]
pub use linux::{install, uninstall};

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn install(_template: &str, _params: &ServiceParams<'_>) -> anyhow::Result<()> {
    anyhow::bail!("service install is only supported on macOS and Linux")
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn uninstall(_params: &ServiceParams<'_>) -> anyhow::Result<()> {
    anyhow::bail!("service uninstall is only supported on macOS and Linux")
}
