//! The runtime, one door per operation.
//!
//! Each door answers with its own message rather than a `ServerMessage` that
//! could have been any of them, so a caller matches on nothing — what it asked
//! for is what it gets back, or an error (RFC 0205).

use crate::sys;
use alloc::{string::String, vec::Vec};
use prost::Message;
use proto::{AgentList, SearchSessionsMsg, SessionHitList, SkillBody, SkillList};

/// Name the other agents in this runtime.
pub fn peers() -> Result<AgentList, String> {
    decode(sys::peers::list()?)
}

/// Search the declaring agent's own past conversations.
pub fn sessions(request: &SearchSessionsMsg) -> Result<SessionHitList, String> {
    let mut encoded = Vec::new();
    request
        .encode(&mut encoded)
        .map_err(|_| String::from("could not encode the request"))?;
    decode(sys::sessions::search(&encoded)?)
}

/// Name the skills this agent may reach.
pub fn skills() -> Result<SkillList, String> {
    decode(sys::skills::list()?)
}

/// Load one skill's instructions.
pub fn skill(name: &str) -> Result<SkillBody, String> {
    decode(sys::skills::get(name)?)
}

fn decode<M: Message + Default>(reply: Vec<u8>) -> Result<M, String> {
    M::decode(&reply[..]).map_err(|_| String::from("could not decode the reply"))
}
