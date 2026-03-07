//! Model download with progress reporting via hf-hub.
//!
//! Pre-downloads model files from HuggingFace into the cache directory
//! controlled by `HF_HOME` so mistralrs finds them without re-downloading.
//! Progress events are sent through an mpsc channel for streaming to clients.

use hf_hub::api::tokio::{ApiBuilder, Progress};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio::{sync::mpsc, time::Instant};

const HF_OFFICIAL: &str = "https://huggingface.co";
const HF_MIRROR: &str = "https://hf-mirror.com";
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Events emitted during model download.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    /// A file download has started.
    FileStart {
        /// Filename within the repo.
        filename: String,
        /// Total size in bytes.
        size: u64,
    },
    /// Incremental download progress (delta, not cumulative).
    Progress {
        /// Bytes downloaded in this chunk (delta).
        bytes: u64,
    },
    /// A file download has completed.
    FileEnd {
        /// Filename within the repo.
        filename: String,
    },
}

/// Progress reporter that sends events through an mpsc channel.
///
/// Throttles `update()` calls to at most once per 100ms across all
/// clones (shared `Instant` via `Arc<Mutex<_>>`). hf-hub clones the
/// progress per parallel download chunk, so the shared clock prevents
/// event flooding.
#[derive(Clone)]
struct ChannelProgress {
    tx: mpsc::UnboundedSender<DownloadEvent>,
    filename: String,
    last_update: Arc<Mutex<Instant>>,
}

impl ChannelProgress {
    fn new(tx: mpsc::UnboundedSender<DownloadEvent>) -> Self {
        Self {
            tx,
            filename: String::new(),
            last_update: Arc::new(Mutex::new(Instant::now())),
        }
    }
}

impl Progress for ChannelProgress {
    async fn init(&mut self, size: usize, filename: &str) {
        self.filename = filename.to_owned();
        *self.last_update.lock().await = Instant::now();
        let _ = self.tx.send(DownloadEvent::FileStart {
            filename: filename.to_owned(),
            size: size as u64,
        });
    }

    async fn update(&mut self, size: usize) {
        let mut last = self.last_update.lock().await;
        let now = Instant::now();
        if now.duration_since(*last).as_millis() >= 100 {
            *last = now;
            drop(last);
            let _ = self.tx.send(DownloadEvent::Progress { bytes: size as u64 });
        }
    }

    async fn finish(&mut self) {
        let _ = self.tx.send(DownloadEvent::FileEnd {
            filename: self.filename.clone(),
        });
    }
}

/// Probe both HuggingFace endpoints and return the faster one.
///
/// Sends a lightweight GET to each endpoint's API and returns whichever
/// responds first. Falls back to the official endpoint if both fail.
pub async fn probe_endpoint() -> String {
    let probe_path = "/api/models/gpt2/revision/main";
    let client = reqwest::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .build()
        .unwrap_or_default();

    let official = {
        let c = client.clone();
        async move { c.get(format!("{HF_OFFICIAL}{probe_path}")).send().await }
    };
    let mirror = {
        let c = client.clone();
        async move { c.get(format!("{HF_MIRROR}{probe_path}")).send().await }
    };

    tokio::select! {
        Ok(_) = official => HF_OFFICIAL.to_owned(),
        Ok(_) = mirror => HF_MIRROR.to_owned(),
        else => HF_OFFICIAL.to_owned(),
    }
}

/// Download all files for a model repo, sending progress events to `tx`.
///
/// Uses hf-hub's async API with `download_with_progress()`. Reads
/// `HF_HOME` and `HF_ENDPOINT` from env (set by `build_provider`).
pub async fn download_model(
    model_id: &str,
    tx: mpsc::UnboundedSender<DownloadEvent>,
) -> anyhow::Result<()> {
    let api = ApiBuilder::from_env().with_progress(false).build()?;
    let repo = api.model(model_id.to_owned());
    let info = repo.info().await?;

    let progress = ChannelProgress::new(tx);
    for sibling in &info.siblings {
        repo.download_with_progress(&sibling.rfilename, progress.clone())
            .await?;
    }

    Ok(())
}
