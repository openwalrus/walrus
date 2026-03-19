//! macOS launchd service management.

use crate::paths::LOGS_DIR;
use crate::service::{ServiceParams, render_template};
use anyhow::Result;

fn launchctl_domain() -> String {
    let uid = std::process::Command::new("id")
        .arg("-u")
        .output()
        .expect("failed to run `id -u`");
    let uid = String::from_utf8_lossy(&uid.stdout).trim().to_string();
    format!("gui/{uid}")
}

pub fn install(template: &str, params: &ServiceParams<'_>) -> Result<()> {
    let plist_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(format!("Library/LaunchAgents/{}.plist", params.label));

    if plist_path.exists() {
        uninstall(params)?;
    }

    let plist = render_template(template, params);

    std::fs::create_dir_all(&*LOGS_DIR)?;

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

pub fn uninstall(params: &ServiceParams<'_>) -> Result<()> {
    let plist_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(format!("Library/LaunchAgents/{}.plist", params.label));

    if !plist_path.exists() {
        anyhow::bail!("service not installed ({})", plist_path.display());
    }

    let service_target = format!("{}/{}", launchctl_domain(), params.label);
    let status = std::process::Command::new("launchctl")
        .args(["bootout", &service_target])
        .status()?;
    if !status.success() {
        eprintln!("warning: launchctl bootout exited with {}", status);
    }

    std::fs::remove_file(&plist_path)?;
    println!("service uninstalled");
    Ok(())
}
