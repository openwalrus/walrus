//! Harness images, one file per digest.
//!
//! An image is immutable under its digest, so re-putting the same bytes
//! rewrites the same file and only the name pointer moves.

use crate::backend::{self, Backend};
use anyhow::Result;
use store::interface::{self, Harnesses};

impl Backend {
    fn image_path(&self, digest: &str) -> std::path::PathBuf {
        self.harnesses_dir().join(backend::encode(digest))
    }

    fn harness_name_path(&self, name: &str) -> std::path::PathBuf {
        self.harnesses_dir()
            .join("names")
            .join(backend::encode(name))
    }
}

impl Harnesses for Backend {
    async fn harness_image(&self, digest: &str) -> Result<Option<Vec<u8>>> {
        Ok(tokio::fs::read(self.image_path(digest)).await.ok())
    }

    async fn put_harness_image(&self, name: &str, bytes: &[u8]) -> Result<String> {
        let digest = interface::digest(bytes);
        tokio::fs::write(self.image_path(&digest), bytes).await?;
        let pointer = self.harness_name_path(name);
        if let Some(parent) = pointer.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(pointer, &digest).await?;
        Ok(digest)
    }

    async fn resolve_harness(&self, name: &str) -> Result<Option<String>> {
        Ok(tokio::fs::read_to_string(self.harness_name_path(name))
            .await
            .ok()
            .map(|digest| digest.trim().to_owned()))
    }
}
