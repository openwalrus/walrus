//! `crabtalk daemon install/uninstall` — system service management.

use anyhow::Result;
use std::path::Path;
use wcore::paths::{HOME_DIR, LOGS_DIR};

#[cfg(target_os = "macos")]
const LAUNCHD_TEMPLATE: &str = include_str!("launchd.plist");
#[cfg(target_os = "linux")]
const SYSTEMD_TEMPLATE: &str = include_str!("systemd.service");

/// Render a template by replacing placeholder tokens.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn render_template(template: &str, binary: &Path) -> String {
    let path = std::env::var("PATH").unwrap_or_default();
    template
        .replace("{binary}", &binary.display().to_string())
        .replace("{logs_dir}", &LOGS_DIR.display().to_string())
        .replace("{home_dir}", &HOME_DIR.display().to_string())
        .replace("{path}", &path)
}

#[cfg(target_os = "macos")]
fn launchctl_domain() -> String {
    let uid = std::process::Command::new("id")
        .arg("-u")
        .output()
        .expect("failed to run `id -u`");
    let uid = String::from_utf8_lossy(&uid.stdout).trim().to_string();
    format!("gui/{uid}")
}

#[cfg(target_os = "macos")]
fn launchctl_service_target() -> String {
    format!("{}/ai.crabtalk.crabtalk", launchctl_domain())
}

#[cfg(target_os = "macos")]
fn plist_path() -> Result<std::path::PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join("Library/LaunchAgents/ai.crabtalk.crabtalk.plist"))
}

#[cfg(target_os = "macos")]
pub fn install() -> Result<()> {
    let plist_path = plist_path()?;

    // Clean up existing installation if present.
    if plist_path.exists() {
        uninstall()?;
    }

    let binary = std::env::current_exe()?;
    let plist = render_template(LAUNCHD_TEMPLATE, &binary);

    std::fs::create_dir_all(&*LOGS_DIR)?;
    std::fs::create_dir_all(&*HOME_DIR)?;

    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&plist_path, plist)?;
    println!("wrote {}", plist_path.display());

    let output = std::process::Command::new("launchctl")
        .args(["bootstrap", &launchctl_domain()])
        .arg(&plist_path)
        .output()?;
    if output.status.success() {
        println!("service loaded and started");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("launchctl bootstrap failed: {stderr}");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn restart() -> Result<()> {
    let plist_path = plist_path()?;

    if !plist_path.exists() {
        anyhow::bail!(
            "service not installed — run `crabtalk daemon install` first, \
             or stop and start the daemon manually"
        );
    }

    // kickstart -k kills the running instance; launchd restarts it (KeepAlive).
    let status = std::process::Command::new("launchctl")
        .args(["kickstart", "-k", &launchctl_service_target()])
        .status()?;
    if status.success() {
        println!("daemon restarted");
    } else {
        anyhow::bail!("launchctl kickstart failed (exit {})", status);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn uninstall() -> Result<()> {
    let plist_path = plist_path()?;

    if !plist_path.exists() {
        anyhow::bail!("service not installed ({})", plist_path.display());
    }

    let status = std::process::Command::new("launchctl")
        .args(["bootout", &launchctl_service_target()])
        .status()?;
    if !status.success() {
        eprintln!("warning: launchctl bootout exited with {}", status);
    }

    std::fs::remove_file(&plist_path)?;
    println!("service uninstalled");
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn install() -> Result<()> {
    let binary = std::env::current_exe()?;
    let unit = render_template(SYSTEMD_TEMPLATE, &binary);

    let unit_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(".config/systemd/user");
    std::fs::create_dir_all(&unit_dir)?;
    std::fs::create_dir_all(&*LOGS_DIR)?;
    std::fs::create_dir_all(&*HOME_DIR)?;

    let unit_path = unit_dir.join("crabtalk-daemon.service");
    std::fs::write(&unit_path, unit)?;
    println!("wrote {}", unit_path.display());

    let status = std::process::Command::new("systemctl")
        .args(["--user", "enable", "--now", "crabtalk-daemon.service"])
        .status()?;
    if status.success() {
        println!("service enabled and started");
    } else {
        anyhow::bail!("systemctl enable failed (exit {})", status);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn restart() -> Result<()> {
    let unit_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(".config/systemd/user/crabtalk-daemon.service");

    if !unit_path.exists() {
        anyhow::bail!(
            "service not installed — run `crabtalk daemon install` first, \
             or stop and start the daemon manually"
        );
    }

    let status = std::process::Command::new("systemctl")
        .args(["--user", "restart", "crabtalk-daemon.service"])
        .status()?;
    if status.success() {
        println!("daemon restarted");
    } else {
        anyhow::bail!("systemctl restart failed (exit {})", status);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn uninstall() -> Result<()> {
    let unit_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(".config/systemd/user/crabtalk-daemon.service");

    if !unit_path.exists() {
        anyhow::bail!("service not installed ({})", unit_path.display());
    }

    let status = std::process::Command::new("systemctl")
        .args(["--user", "disable", "--now", "crabtalk-daemon.service"])
        .status()?;
    if !status.success() {
        eprintln!("warning: systemctl disable exited with {}", status);
    }

    std::fs::remove_file(&unit_path)?;
    println!("service uninstalled");
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn install() -> Result<()> {
    anyhow::bail!("service install is only supported on macOS and Linux")
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn restart() -> Result<()> {
    anyhow::bail!("service restart is only supported on macOS and Linux")
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn uninstall() -> Result<()> {
    anyhow::bail!("service uninstall is only supported on macOS and Linux")
}
