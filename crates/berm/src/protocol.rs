//! The runtime, as a harness sees it.
//!
//! One door per operation, so what a harness may ask is decided by which doors
//! it was linked to rather than by an allowlist run over a decoded payload.
//! `sessions::search` cannot list agents the way one `ClientMessage` door could
//! be talked into doing — there is no field to inspect, only a function that
//! was or was not registered.
//!
//! Each door also owns its own reply, which is where narrowing lives.
//! `AgentInfo.config` is the full `AgentConfig` as JSON and an agent's config
//! holds its MCPs by value — `env` and a literal `Authorization` header among
//! them — so `peers` returns a list with those fields cleared. That is this
//! boundary paying for a bill RFC 0193 deferred, and it holds the line rather
//! than settling it.

use crate::sys;
use anyhow::{Context as _, Result, bail};
use berm::Harness;
use prost::Message;
use proto::{
    ClientMessage, GetSkillMsg, ListAgentsMsg, ListSkillsMsg, SearchSessionsMsg, ServerMessage,
    client_message, server_message,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, OnceLock},
};
use store::AgentId;
use tokio::runtime::Handle;

/// How a harness reaches the runtime.
///
/// `Server::dispatch` is already the one door, but the trait is not
/// object-safe, so the daemon hands over a closure rather than itself — which
/// also keeps this crate from depending on the one that implements it.
pub type Dispatch = Arc<
    dyn Fn(ClientMessage) -> Pin<Box<dyn Future<Output = Vec<ServerMessage>> + Send>> + Send + Sync,
>;

/// What the agent behind a harness narrows the runtime to.
///
/// The agent's own limits are known here without the invocation having to carry
/// them. That is also why this is part of an image's digest: two agents
/// declaring the same ELF under different scopes are two sandboxes, not one.
pub struct Scope {
    /// The skills this agent declared. Empty is unrestricted, which is what
    /// an agent naming none has always meant.
    pub skills: Vec<String>,
    /// The agent that declared the harness. Session search is narrowed to it
    /// rather than filtered by it: a harness asking for someone else's
    /// conversations is answered about its own.
    pub agent: AgentId,
}

impl Scope {
    /// Whether `name` is a skill this agent may reach.
    fn may_use(&self, name: &str) -> bool {
        self.skills.is_empty() || self.skills.iter().any(|s| s == name)
    }
}

/// What the runtime's doors are served by.
pub struct Protocol {
    /// The dispatcher arrives after the harnesses do — the daemon that
    /// implements it is built on top of them — so it is read through a
    /// `OnceLock` rather than held. A call before it is connected fails
    /// rather than waiting.
    dispatch: Arc<OnceLock<Dispatch>>,
    /// Held rather than looked up per call, for the reason [`crate::Http`]
    /// holds one: the runtime is async, the sandbox is sync, and which
    /// reactor bridges them is the embedder's to decide once.
    reactor: Handle,
    scope: Scope,
}

impl Protocol {
    pub fn new(dispatch: Arc<OnceLock<Dispatch>>, reactor: Handle, scope: Scope) -> Self {
        Self {
            dispatch,
            reactor,
            scope,
        }
    }

    /// Every door the runtime opens. Their names come from the declaration
    /// `berm-crabtalk` builds its stubs from, so there is no string here for
    /// the two sides to disagree about.
    pub fn harnesses(self) -> Vec<Harness> {
        let peers = Arc::new(self);
        let (sessions, list, get) = (peers.clone(), peers.clone(), peers.clone());
        vec![
            sys::peers::list(move || peers.peers()),
            sys::sessions::search(move |request| sessions.sessions(request)),
            sys::skills::list(move || list.skills()),
            sys::skills::get(move |name| get.skill(name)),
        ]
    }

    /// Name the other agents, without the configs that carry their credentials.
    fn peers(&self) -> Result<Vec<u8>> {
        let reply = self.ask(client_message::Msg::ListAgents(ListAgentsMsg {}))?;
        let Some(server_message::Msg::AgentList(mut list)) = reply.msg else {
            bail!("the runtime did not return an agent list");
        };
        for info in &mut list.agents {
            info.config.clear();
        }
        encode(&list)
    }

    /// Search the declaring agent's own conversations.
    fn sessions(&self, request: &[u8]) -> Result<Vec<u8>> {
        let mut message = SearchSessionsMsg::decode(request)?;

        // Overwritten rather than checked: the agent filter is not the
        // harness's to choose, and refusing a wrong one would only teach it to
        // send the right one. `sender` stays free — an agent's own
        // conversations span every partner it has, and it can already resume
        // any of them.
        message.agent = self.scope.agent.to_string();

        let reply = self.ask(client_message::Msg::SearchSessions(message))?;
        let Some(server_message::Msg::SessionHits(hits)) = reply.msg else {
            bail!("the runtime did not return session hits");
        };
        encode(&hits)
    }

    /// Name the skills, dropping what the agent did not declare.
    fn skills(&self) -> Result<Vec<u8>> {
        let reply = self.ask(client_message::Msg::ListSkills(ListSkillsMsg {}))?;
        let Some(server_message::Msg::SkillList(mut list)) = reply.msg else {
            bail!("the runtime did not return a skill list");
        };
        list.skills.retain(|skill| self.scope.may_use(&skill.name));
        encode(&list)
    }

    /// One skill's instructions.
    fn skill(&self, name: &str) -> Result<Vec<u8>> {
        // Refused here rather than filtered out of the reply, so asking for a
        // skill outside the declaration costs nothing and says so.
        if !self.scope.may_use(name) {
            bail!("skill not available: {name}");
        }
        let reply = self.ask(client_message::Msg::GetSkill(GetSkillMsg {
            name: name.to_owned(),
        }))?;
        let Some(server_message::Msg::SkillBody(body)) = reply.msg else {
            bail!("the runtime did not return a skill body");
        };
        encode(&body)
    }

    /// Put one message to the runtime and take its answer.
    ///
    /// Every message a door carries is request-response — a streaming one is
    /// behind no door — so one reply is the whole answer.
    fn ask(&self, message: client_message::Msg) -> Result<ServerMessage> {
        let Some(dispatch) = self.dispatch.get() else {
            bail!("the protocol is not connected yet");
        };
        let replies = self
            .reactor
            .block_on(dispatch(ClientMessage { msg: Some(message) }));
        replies
            .into_iter()
            .next()
            .context("the runtime returned no reply")
    }
}

fn encode(message: &impl Message) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    message.encode(&mut encoded)?;
    Ok(encoded)
}
