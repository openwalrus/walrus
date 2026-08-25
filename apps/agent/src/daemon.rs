//! The daemon: a store, the runtime over it, and the sockets it answers on.

use crate::backend::Backend;
use crate::serve;
use anyhow::Result;
use crabtalk::{Config, CrabTalk};
use std::{sync::Arc, time::Duration};

/// How long a listener gets to drain after shutdown is broadcast.
const DRAIN: Duration = Duration::from_secs(5);

/// What this install starts the daemon with: the settings as written, and
/// the two directories only an installed process can name.
fn config() -> Result<Config> {
    Ok(Config {
        settings: store::Config::load(&crabup::dirs::CONFIG_FILE)?,
        harnesses: crabup::dirs::HARNESSES_DIR.clone(),
        cache: crabup::dirs::CACHE_DIR.join("berm"),
    })
}

pub async fn start() -> Result<()> {
    let config = config()?;
    relocate()?;
    let store = Arc::new(Backend::open(&*crabup::dirs::STORE_FILE)?);
    tracing::info!("store at {}", crabup::dirs::STORE_FILE.display());

    let handle = CrabTalk::start(config, store.clone()).await?;

    #[cfg(unix)]
    let (socket, socket_join) = serve::socket(handle.inner.clone(), &handle.shutdown_tx)?;
    let (tcp_join, port) = serve::tcp(handle.inner.clone(), &handle.shutdown_tx)?;
    std::fs::write(&*crabup::dirs::PORT_FILE, port.to_string())?;

    handle.wait_until_ready().await?;
    tracing::info!("daemon ready");

    terminate().await?;
    tracing::info!("shutting down");
    handle.shutdown().await?;

    #[cfg(unix)]
    {
        let _ = tokio::time::timeout(DRAIN, socket_join).await;
        let _ = std::fs::remove_file(socket);
    }
    let _ = tokio::time::timeout(DRAIN, tcp_join).await;
    let _ = std::fs::remove_file(&*crabup::dirs::PORT_FILE);

    store.checkpoint()
}

/// Move a store written before there was a directory to share.
///
/// The daemon is no longer the only thing here that keeps one, so its own
/// went from the install root into `store/`. An install that predates
/// that has agents and sessions in the old place, and leaving them there
/// would read as having lost them.
fn relocate() -> Result<()> {
    let (from, to) = (
        &*crabup::dirs::LEGACY_STORE_FILE,
        &*crabup::dirs::STORE_FILE,
    );
    if !from.exists() || to.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(&*crabup::dirs::STORE_DIR)?;
    std::fs::rename(from, to)?;
    tracing::info!("moved the store to {}", to.display());
    Ok(())
}

/// SIGTERM is how whatever spawned this kills it; ctrl-c is the terminal case.
#[cfg(unix)]
async fn terminate() -> Result<()> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut term = signal(SignalKind::terminate())?;
    tokio::select! {
        signal = tokio::signal::ctrl_c() => signal?,
        _ = term.recv() => {}
    }
    Ok(())
}

#[cfg(not(unix))]
async fn terminate() -> Result<()> {
    Ok(tokio::signal::ctrl_c().await?)
}
