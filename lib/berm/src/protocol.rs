//! The runtime, one door per operation.
//!
//! Each door answers with its own message rather than a `ServerMessage` that
//! could have been any of them, so a caller matches on nothing — what it asked
//! for is what it gets back, or an error (RFC 0205).

use crate::sys;
use alloc::{string::String, vec::Vec};
use berm_lang::CallError;
use prost::Message;
use proto::{AgentList, SearchSessionsMsg, SessionHitList, SkillBody, SkillList};

/// Name the other agents in this runtime.
pub fn peers() -> Result<AgentList, String> {
    decode(sys::peers::list().map_err(message)?)
}

/// Search the declaring agent's own past conversations.
pub fn sessions(request: &SearchSessionsMsg) -> Result<SessionHitList, String> {
    let mut encoded = Vec::new();
    request
        .encode(&mut encoded)
        .map_err(|_| String::from("could not encode the request"))?;
    decode(sys::sessions::search(&encoded).map_err(message)?)
}

/// Name the skills this agent may reach.
pub fn skills() -> Result<SkillList, String> {
    decode(sys::skills::list().map_err(message)?)
}

/// Load one skill's instructions.
pub fn skill(name: &str) -> Result<SkillBody, String> {
    decode(sys::skills::get(name).map_err(message)?)
}

fn decode<M: Message + Default>(reply: Vec<u8>) -> Result<M, String> {
    M::decode(&reply[..]).map_err(|_| String::from("could not decode the reply"))
}

/// Both kinds of [`CallError`] collapse to their message here: a harness
/// reading its own runtime door has nothing useful to do with a refusal that
/// a normal failure wouldn't already tell it.
fn message(error: CallError) -> String {
    String::from(error.message())
}
