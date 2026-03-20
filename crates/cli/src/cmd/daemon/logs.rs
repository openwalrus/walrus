//! `crabtalk daemon logs` — view daemon logs.

use anyhow::Result;

/// Display daemon log output by delegating to the shared `crabtalk_command::view_logs`.
pub fn logs(tail_args: &[String]) -> Result<()> {
    crabtalk_command::view_logs("daemon", tail_args)
}
