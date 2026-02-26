//! Gateway server singleton — manages the embedded tokio runtime and gateway.

use anyhow::Result;
use gateway::ServeHandle;
use std::path::Path;
use std::sync::Mutex;

/// Global gateway state. `None` when stopped, `Some` when running.
static SERVER: Mutex<Option<GatewayServer>> = Mutex::new(None);

/// Holds the tokio runtime and the gateway serve handle.
struct GatewayServer {
    runtime: tokio::runtime::Runtime,
    handle: ServeHandle,
    port: u16,
}

/// Start the embedded gateway on `127.0.0.1:0` (OS-assigned port).
///
/// Returns the bound port on success.
pub fn start(config_dir: &Path) -> Result<u16> {
    let mut guard = SERVER
        .lock()
        .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
    if guard.is_some() {
        anyhow::bail!("gateway already running");
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let handle = runtime.block_on(gateway::serve(config_dir, "127.0.0.1:0"))?;
    let port = handle.port;

    *guard = Some(GatewayServer {
        runtime,
        handle,
        port,
    });

    Ok(port)
}

/// Stop the embedded gateway and drop the tokio runtime.
pub fn stop() -> Result<()> {
    let mut guard = SERVER
        .lock()
        .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
    let server = guard
        .take()
        .ok_or_else(|| anyhow::anyhow!("gateway not running"))?;

    let GatewayServer {
        runtime, handle, ..
    } = server;

    runtime.block_on(handle.shutdown())?;
    Ok(())
}

/// Query the current gateway port. Returns 0 if not running.
pub fn port() -> u16 {
    SERVER
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.port))
        .unwrap_or(0)
}
