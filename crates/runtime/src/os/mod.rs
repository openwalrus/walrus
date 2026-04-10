//! OS utilities for agent environment prompts.

use std::fmt::Write;

/// Build an `<environment>` XML block with OS info.
pub fn environment_block() -> String {
    let mut buf = String::from("\n\n<environment>\n");
    let _ = writeln!(buf, "os: {}", std::env::consts::OS);
    buf.push_str("</environment>");
    buf
}
