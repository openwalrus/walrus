//! Gateway server singleton — manages the embedded tokio runtime and gateway.

use anyhow::Result;
use gateway::ServeHandle;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Global gateway state. `None` when stopped, `Some` when running.
static SERVER: Mutex<Option<GatewayServer>> = Mutex::new(None);

/// Holds the tokio runtime and the gateway serve handle.
struct GatewayServer {
    runtime: tokio::runtime::Runtime,
    handle: ServeHandle,
    socket_path: PathBuf,
}

/// Start the embedded gateway on a Unix domain socket in the config directory.
///
/// Returns the socket path on success.
pub fn start(config_dir: &Path) -> Result<PathBuf> {
    let mut guard = SERVER
        .lock()
        .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
    if guard.is_some() {
        anyhow::bail!("gateway already running");
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let handle = runtime.block_on(gateway::serve(config_dir, None))?;
    let socket_path = handle.socket_path.clone();

    *guard = Some(GatewayServer {
        runtime,
        handle,
        socket_path: socket_path.clone(),
    });

    Ok(socket_path)
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

/// Query the current gateway socket path. Returns `None` if not running.
pub fn socket_path() -> Option<CString> {
    SERVER
        .lock()
        .ok()
        .and_then(|g| {
            g.as_ref()
                .and_then(|s| CString::new(s.socket_path.to_string_lossy().as_ref()).ok())
        })
}
