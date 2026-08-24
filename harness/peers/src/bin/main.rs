//! Name the other agents in the runtime.
//!
//! The smallest thing that reaches the runtime rather than the machine: one
//! tool, one door. It exists to exercise that door end to end — being linked
//! to it at all, and the redaction on the way back — with nothing in the way.
//!
//! Naming the peers is all it does. Reaching one is a turn spent on another
//! agent's behalf, and the runtime opens no door onto that.

#![cfg_attr(target_arch = "riscv64", no_std, no_main)]

extern crate alloc;

#[berm_lang::harness]
mod tools {
    use berm_crabtalk::protocol;
    use berm_lang::{Failed, Out};
    use core::fmt::Write;

    /// List the other agents in this runtime, with their descriptions.
    pub fn peers(_args: &[u8], out: &mut Out) -> Result<(), Failed> {
        let list = match protocol::peers() {
            Ok(list) => list,
            Err(error) => {
                out.write(error.as_bytes());
                return Err(Failed);
            }
        };

        for agent in &list.agents {
            let _ = writeln!(out, "{}\t{}", agent.name, agent.description);
        }
        Ok(())
    }
}
