//! Service lifecycle ops for a registry [`Entry`].

use anyhow::{Context, Result, bail};

use crate::registry::Entry;

/// [`command::Service`] adapter for a registry entry whose `label` is `Some`.
/// Non-serviceable entries never reach here.
struct Spec<'a> {
    entry: &'a Entry,
    label: &'a str,
}

impl command::Service for Spec<'_> {
    fn name(&self) -> &str {
        self.entry.short
    }
    fn description(&self) -> &str {
        self.entry.description
    }
    fn label(&self) -> &str {
        self.label
    }
}

impl Entry {
    fn spec(&self) -> Result<Spec<'_>> {
        let label = self
            .label
            .with_context(|| format!("{} is not a service", self.short))?;
        Ok(Spec { entry: self, label })
    }

    fn require_binary(&self) -> Result<std::path::PathBuf> {
        self.binary_path().with_context(|| {
            format!(
                "{} not installed — run `crabup pull {}` first",
                self.krate, self.short
            )
        })
    }

    pub fn start(&self, force: bool) -> Result<()> {
        let spec = self.spec()?;
        if !force && command::is_installed(spec.label) {
            println!("{} is already running", self.short);
            return Ok(());
        }
        let binary = self.require_binary()?;
        let rendered = command::render_service_template(&spec, &binary);
        command::install(&rendered, spec.label)?;
        println!("started {}", self.short);
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let spec = self.spec()?;
        if !command::is_installed(spec.label) {
            println!("{} is not running", self.short);
            return Ok(());
        }
        command::uninstall(spec.label)?;
        let _ = std::fs::remove_file(wcore::paths::service_port_file(self.short));
        println!("stopped {}", self.short);
        Ok(())
    }

    pub fn restart(&self) -> Result<()> {
        let spec = self.spec()?;
        if command::is_installed(spec.label) {
            command::uninstall(spec.label)?;
        }
        let binary = self.require_binary()?;
        let rendered = command::render_service_template(&spec, &binary);
        command::install(&rendered, spec.label)?;
        println!("restarted {}", self.short);
        Ok(())
    }

    pub fn logs(&self, tail_args: &[String]) -> Result<()> {
        if self.label.is_none() {
            bail!("{} is not a service", self.short);
        }
        command::view_logs(self.short, tail_args)
    }
}
