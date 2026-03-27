//! Windows service management.
//!
//! Currently a stub — the daemon runs in the foreground on Windows.
//! A future implementation could use Task Scheduler or the `windows-service` crate.

use anyhow::Result;

pub fn is_installed(_label: &str) -> bool {
    false
}

pub fn install(_rendered: &str, _label: &str) -> Result<()> {
    anyhow::bail!(
        "service install is not yet supported on Windows. \
         Run the daemon in the foreground with `crabtalk daemon run`."
    )
}

pub fn uninstall(_label: &str) -> Result<()> {
    anyhow::bail!(
        "service uninstall is not yet supported on Windows. \
         Stop the daemon with Ctrl+C."
    )
}
