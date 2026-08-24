//! `berm.call` — one harness reaching another.
//!
//! berm's own name rather than a `crabtalk` one: the guest reaches it through
//! [`berm_lang::call`], which is about the harness model itself rather than
//! about this host's world. berm serves it for nobody — a [`berm::Berm`] is one
//! harness with nothing to dispatch to — so a host running more than one is
//! what registers it.
//!
//! The target is named at the call, not at load. Closing each image over a
//! table of its siblings would be the shape every other door here has, and it
//! cannot express a cycle: two harnesses that call each other cannot both be
//! built first. Resolving against the registry per call also means a
//! declaration added later is reachable without recompiling the images that
//! were already loaded.
//!
//! Which resolution the name is looked up in is not a permission. A name alone
//! does not identify an image — the registry is keyed by digest, and the same
//! name means different bytes under different agents and roots — so the
//! caller's own resolution is the only reading of it that is unambiguous.

use crate::harness::Registry;
use berm::{Harness, Refused, abi, wire};
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};
use store::AgentId;

/// Serve `berm.call` for one image, against the resolution it was built for.
pub fn harness(
    registry: Arc<RwLock<Registry>>,
    agent: AgentId,
    session: Option<PathBuf>,
) -> Harness {
    Harness {
        name: abi::CALL.to_owned(),
        call: Arc::new(move |request: &[u8]| {
            let fields = wire::fields(request)?;
            let name = wire::text(&fields, 0, "harness")?;
            let tool = wire::text(&fields, 1, "tool")?;
            let args = wire::text(&fields, 2, "arguments")?;

            // Cloned out from under the guard, which is then dropped: entering
            // the target takes this lock again for its own calls, and a second
            // read behind a waiting writer is a deadlock.
            let target = registry
                .read()
                .expect("harness registry")
                .named(&agent, session.as_deref(), name)
                .cloned();

            let Some(target) = target else {
                return Err(Refused(format!("no harness named {name:?} is declared here")).into());
            };
            if !target.tools().any(|declared| declared == tool) {
                return Err(Refused(format!("{name} exports no tool named {tool:?}")).into());
            }

            match target.call(tool, args.as_bytes().to_vec())? {
                Ok(result) => Ok(result.into_bytes()),
                Err(failure) => anyhow::bail!(failure),
            }
        }),
    }
}
