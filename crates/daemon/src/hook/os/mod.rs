//! OS hook — shell tool for agents.
//!
//! Registers the `bash` tool schema. Dispatch method lives on
//! [`DaemonHook`](crate::hook::DaemonHook).

use std::fmt::Write;

pub(crate) mod tool;

/// Build an `<environment>` XML block with OS and working directory.
/// Appended to every agent's system prompt.
pub fn environment_block(cwd: &std::path::Path) -> String {
    let mut buf = String::from("\n\n<environment>\n");
    let _ = writeln!(buf, "os: {}", std::env::consts::OS);
    let _ = writeln!(buf, "working_directory: {}", cwd.display());
    buf.push_str("</environment>");
    buf
}
