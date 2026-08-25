//! Standing subscriptions, in one file.
//!
//! They are rewritten whenever one is added, dropped, or fires once, so
//! there is nothing an append would buy over a whole document.

use crate::backend::{self, Backend};
use anyhow::Result;
use store::{EventSubscription, interface::Subscriptions};

impl Backend {
    fn subscriptions_path(&self) -> std::path::PathBuf {
        self.root.join("subscriptions.json")
    }
}

impl Subscriptions for Backend {
    async fn subscriptions(&self) -> Result<Vec<EventSubscription>> {
        Ok(backend::read_json(&self.subscriptions_path())
            .await?
            .unwrap_or_default())
    }

    async fn put_subscription(&self, sub: &EventSubscription) -> Result<()> {
        let mut subs = self.subscriptions().await?;
        match subs.iter_mut().find(|held| held.id == sub.id) {
            Some(held) => *held = sub.clone(),
            None => subs.push(sub.clone()),
        }
        backend::write_json(&self.subscriptions_path(), &subs).await
    }

    async fn remove_subscription(&self, id: u64) -> Result<bool> {
        let mut subs = self.subscriptions().await?;
        let before = subs.len();
        subs.retain(|sub| sub.id != id);
        if subs.len() == before {
            return Ok(false);
        }
        backend::write_json(&self.subscriptions_path(), &subs).await?;
        Ok(true)
    }
}
