//! Both ends of a nested call, in one image.
//!
//! `nest` reaches another harness by name; `echo` and `boom` are what a name
//! can resolve to. One ELF deployed under two names is enough to drive every
//! outcome `berm.call` carries, which is why this is not four crates.
//!
//! Not installed by `make harness`. It exists for `cargo run --example call
//! -p crabtalk-berm`, and nothing in a running daemon declares it.

#![cfg_attr(target_arch = "riscv64", no_std, no_main)]

extern crate alloc;

#[berm_lang::harness]
mod tools {
    use alloc::string::String;
    use berm_lang::{CallError, Failed, Out, tool::parse};
    use core::fmt::Write;

    /// Call a tool on another harness and report what came back.
    #[args(Nest)]
    pub fn nest(args: &[u8], out: &mut Out) -> Result<(), Failed> {
        let input: Nest = parse(args, out)?;
        let forwarded = input.args.as_deref().unwrap_or("{}");

        // The two failures are written apart rather than collapsed, because
        // telling them apart is the whole of what this fixture checks.
        match berm_lang::call(&input.harness, &input.tool, forwarded) {
            Ok(result) => {
                let _ = write!(out, "reached: {result}");
                Ok(())
            }
            Err(CallError::Refused(message)) => {
                let _ = write!(out, "refused: {message}");
                Err(Failed)
            }
            Err(CallError::Failed(message)) => {
                let _ = write!(out, "failed: {message}");
                Err(Failed)
            }
        }
    }

    /// Answer with the arguments, so a caller can see the payload survived.
    pub fn echo(args: &[u8], out: &mut Out) -> Result<(), Failed> {
        out.write(args);
        Ok(())
    }

    /// Always report failure — a target that ran and said no.
    pub fn boom(_args: &[u8], out: &mut Out) -> Result<(), Failed> {
        out.write(b"boom");
        Err(Failed)
    }

    /// Arguments for `nest`.
    pub struct Nest {
        /// Name the target is deployed under.
        pub harness: String,
        /// Tool to run on it.
        pub tool: String,
        /// Argument blob to forward, as JSON text. Defaults to `{}`.
        pub args: Option<String>,
    }
}
