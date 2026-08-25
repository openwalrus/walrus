//! The coding agent: a runtime in this process, rooted where it started.
//!
//! No socket and no daemon. `crab` embeds the runtime the way the daemon
//! does and skips the listeners, so the only thing between a keystroke
//! and a tool call is a function call.

use crate::backend::Backend;
use anyhow::{Result, bail};
use crabtalk::{Config, CrabTalk, CrabTalkHandle, system::provider::DefaultProvider};
use futures_util::StreamExt;
use proto::{CreateSessionMsg, StreamMsg, server::Server, stream_event::Event};
use std::{path::PathBuf, sync::Arc};
use store::{
    AgentId, SessionHandle,
    interface::{Agents, Sessions},
};
use tokio::sync::mpsc;

/// Where crab keeps its own state, beside the daemon's rather than in it.
const STORE_DIR: &str = "store/code";

pub struct Crab {
    pub handle: CrabTalkHandle<DefaultProvider, Backend>,
    pub agent: AgentId,
    pub session: SessionHandle,
    pub root: PathBuf,
}

impl Crab {
    /// Start the runtime over `root`, resuming the session that directory
    /// already has.
    ///
    /// The handle is the root's own path: running `crab` twice in one
    /// checkout continues the conversation rather than starting a second.
    pub async fn open(root: PathBuf) -> Result<Self> {
        let config = Config {
            settings: store::Config::load(&crabup::dirs::CONFIG_FILE)?,
            harnesses: crabup::dirs::HARNESSES_DIR.clone(),
            cache: crabup::dirs::CACHE_DIR.join("berm"),
        };
        let store = Arc::new(Backend::open(crabup::dirs::CONFIG_DIR.join(STORE_DIR))?);
        let handle = CrabTalk::start(config, store.clone()).await?;

        let Some(agent) = store.default_agent().await? else {
            bail!("no default agent — the install did not scaffold one");
        };
        let session = SessionHandle::new(root.display().to_string());
        if store.load_session(&session).await?.is_none() {
            handle
                .inner
                .create_session(CreateSessionMsg {
                    session_handle: session.as_str().to_owned(),
                    agent: agent.to_string(),
                    sender: None,
                    root: Some(root.display().to_string()),
                })
                .await?;
        }

        Ok(Self {
            handle,
            agent,
            session,
            root,
        })
    }

    /// Start a turn, delivering its events on a channel.
    ///
    /// Spawned rather than returned as a stream so the caller can hold it
    /// across a select loop, and drop it to walk away from a turn.
    pub fn spawn_turn(&self, prompt: &str) -> mpsc::UnboundedReceiver<Result<Event>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let (crabtalk, req) = (
            self.handle.inner.clone(),
            StreamMsg {
                agent: self.agent.to_string(),
                content: prompt.to_owned(),
                session_handle: self.session.as_str().to_owned(),
                ..Default::default()
            },
        );
        tokio::spawn(async move {
            let mut stream = std::pin::pin!(crabtalk.stream(req));
            while let Some(event) = stream.next().await {
                let event = match event {
                    Ok(event) => event.event,
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                };
                if let Some(event) = event
                    && tx.send(Ok(event)).is_err()
                {
                    return;
                }
            }
        });
        rx
    }

    /// Run one turn, writing the stream to stdout as it arrives.
    pub async fn turn(&self, prompt: &str) -> Result<()> {
        use std::io::Write;

        let mut events = self.spawn_turn(prompt);
        while let Some(event) = events.recv().await {
            let event = event?;
            match event {
                Event::Chunk(chunk) => {
                    print!("{}", chunk.content);
                    std::io::stdout().flush()?;
                }
                Event::ToolStart(start) => {
                    for call in &start.calls {
                        println!("\n⏺ {}", call.name);
                    }
                }
                Event::End(end) if !end.error.is_empty() => bail!("{}", end.error),
                Event::End(_) => println!(),
                _ => {}
            }
        }
        Ok(())
    }
}
