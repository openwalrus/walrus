//! Runtime event bus — subscription-based routing to agents.
//!
//! Subscriptions match on an exact `source` string. When a matching
//! event is published, the bus invokes a user-supplied `fire` callback
//! with the subscription and payload; `once` subscriptions are dropped
//! after firing. The bus owns zero knowledge of how the callback
//! delivers the message — the daemon wires it up to forward into its
//! event loop.
//!
//! Persistence goes through the [`Storage`] trait at the key
//! `events/subscriptions.toml`. Memory is authoritative at runtime;
//! disk is recovery for restarts.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use wcore::Storage;

/// Storage key for the subscription TOML blob.
pub const SUBSCRIPTIONS_KEY: &str = "events/subscriptions.toml";

/// Persistent event subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    pub id: u64,
    pub source: String,
    pub target_agent: String,
    #[serde(default)]
    pub once: bool,
}

/// TOML file wrapper — `[[subscription]]` array of tables.
#[derive(Debug, Default, Serialize, Deserialize)]
struct EventFile {
    #[serde(default)]
    subscription: Vec<EventSubscription>,
}

/// Callback signature for firing a matched subscription. The daemon
/// supplies this closure at construction time — the event bus itself
/// has no opinion on how the resulting message is delivered.
pub type FireCallback = Arc<dyn Fn(&EventSubscription, &str) + Send + Sync>;

/// In-memory event bus with subscription routing + Storage-backed
/// recovery.
pub struct EventBus {
    subscriptions: HashMap<u64, EventSubscription>,
    next_id: u64,
    storage: Arc<dyn Storage>,
    fire: FireCallback,
}

impl EventBus {
    /// Load subscriptions from [`SUBSCRIPTIONS_KEY`] in the given
    /// [`Storage`]. Missing or unparsable blobs are tolerated — they
    /// yield an empty subscription set.
    pub fn load(storage: Arc<dyn Storage>, fire: FireCallback) -> Self {
        let mut subscriptions = HashMap::new();
        let mut max_id = 0u64;
        match storage.get(SUBSCRIPTIONS_KEY) {
            Ok(Some(bytes)) => match std::str::from_utf8(&bytes) {
                Ok(content) => match toml::from_str::<EventFile>(content) {
                    Ok(file) => {
                        for sub in file.subscription {
                            max_id = max_id.max(sub.id);
                            subscriptions.insert(sub.id, sub);
                        }
                    }
                    Err(e) => tracing::warn!(
                        "failed to parse {SUBSCRIPTIONS_KEY}, starting with empty subscriptions: {e}"
                    ),
                },
                Err(_) => {
                    tracing::warn!("{SUBSCRIPTIONS_KEY} is not valid UTF-8, ignoring");
                }
            },
            Ok(None) => {}
            Err(e) => tracing::warn!("failed to read {SUBSCRIPTIONS_KEY}: {e}"),
        }
        Self {
            subscriptions,
            next_id: max_id + 1,
            storage,
            fire,
        }
    }

    /// Create a new subscription. Assigns an ID and persists.
    pub fn subscribe(&mut self, mut sub: EventSubscription) -> EventSubscription {
        sub.id = self.next_id;
        self.next_id += 1;
        self.subscriptions.insert(sub.id, sub.clone());
        self.save();
        sub
    }

    /// Remove a subscription. Returns whether it existed.
    pub fn unsubscribe(&mut self, id: u64) -> bool {
        if self.subscriptions.remove(&id).is_none() {
            return false;
        }
        self.save();
        true
    }

    /// List all subscriptions.
    pub fn list(&self) -> Vec<EventSubscription> {
        self.subscriptions.values().cloned().collect()
    }

    /// Publish an event. Fires every subscription whose `source` is an
    /// exact match; `once` subscriptions are dropped after firing.
    pub fn publish(&mut self, source: &str, payload: &str) {
        let mut to_remove = Vec::new();
        for (id, sub) in &self.subscriptions {
            if sub.source == source {
                (self.fire)(sub, payload);
                if sub.once {
                    to_remove.push(*id);
                }
            }
        }
        if !to_remove.is_empty() {
            for id in &to_remove {
                self.subscriptions.remove(id);
            }
            self.save();
        }
    }

    /// Write-through to Storage. Serialization failures are logged and
    /// swallowed — the in-memory state remains authoritative.
    fn save(&self) {
        let file = EventFile {
            subscription: self.subscriptions.values().cloned().collect(),
        };
        let content = match toml::to_string_pretty(&file) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to serialize subscriptions: {e}");
                return;
            }
        };
        if let Err(e) = self.storage.put(SUBSCRIPTIONS_KEY, content.as_bytes()) {
            tracing::error!("failed to write {SUBSCRIPTIONS_KEY}: {e}");
        }
    }
}
