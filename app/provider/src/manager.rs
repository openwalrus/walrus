//! `ProviderManager` — concurrent-safe named provider map with active-provider
//! swapping (DD#65).

use crate::{Provider, ProviderConfig, build_provider};
use anyhow::{Result, bail};
use compact_str::CompactString;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// Manages a set of named providers with an active selection.
///
/// All methods that read or mutate the inner state acquire the `RwLock`.
/// `active()` returns a clone of the current `Provider` — callers do not
/// hold the lock while performing LLM calls.
pub struct ProviderManager {
    inner: Arc<RwLock<Inner>>,
}

struct Inner {
    /// Named provider instances.
    providers: BTreeMap<CompactString, Provider>,
    /// Name of the currently active provider.
    active: CompactString,
    /// Shared HTTP client for constructing new providers.
    client: llm::Client,
}

/// Info about a single provider entry returned by `list()`.
#[derive(Debug, Clone)]
pub struct ProviderEntry {
    /// Provider name.
    pub name: CompactString,
    /// Whether this is the active provider.
    pub active: bool,
}

impl ProviderManager {
    /// Create a new manager from a list of provider configs.
    ///
    /// The first config in the list becomes the active provider. Returns an
    /// error if the list is empty or any provider fails to build.
    pub async fn from_configs(configs: &[ProviderConfig]) -> Result<Self> {
        if configs.is_empty() {
            bail!("at least one provider config is required");
        }

        let client = llm::Client::new();
        let mut providers = BTreeMap::new();
        let active = configs[0].name.clone();

        for config in configs {
            let provider = build_provider(config, client.clone()).await?;
            providers.insert(config.name.clone(), provider);
        }

        Ok(Self {
            inner: Arc::new(RwLock::new(Inner {
                providers,
                active,
                client,
            })),
        })
    }

    /// Create a manager with a single provider.
    pub fn single(name: CompactString, provider: Provider) -> Self {
        let mut providers = BTreeMap::new();
        providers.insert(name.clone(), provider);
        Self {
            inner: Arc::new(RwLock::new(Inner {
                providers,
                active: name,
                client: llm::Client::new(),
            })),
        }
    }

    /// Get a clone of the active provider.
    pub fn active(&self) -> Provider {
        let inner = self.inner.read().expect("provider lock poisoned");
        inner.providers[&inner.active].clone()
    }

    /// Get the name of the active provider.
    pub fn active_name(&self) -> CompactString {
        let inner = self.inner.read().expect("provider lock poisoned");
        inner.active.clone()
    }

    /// Switch to a different provider by name. Returns an error if the name
    /// is not found.
    pub fn switch(&self, name: &str) -> Result<()> {
        let mut inner = self.inner.write().expect("provider lock poisoned");
        if !inner.providers.contains_key(name) {
            bail!("provider '{}' not found", name);
        }
        inner.active = CompactString::from(name);
        Ok(())
    }

    /// Add a new provider. Replaces any existing provider with the same name.
    pub async fn add(&self, config: &ProviderConfig) -> Result<()> {
        let client = {
            let inner = self.inner.read().expect("provider lock poisoned");
            inner.client.clone()
        };
        let provider = build_provider(config, client).await?;
        let mut inner = self.inner.write().expect("provider lock poisoned");
        inner.providers.insert(config.name.clone(), provider);
        Ok(())
    }

    /// Remove a provider by name. Fails if the provider is currently active.
    pub fn remove(&self, name: &str) -> Result<()> {
        let mut inner = self.inner.write().expect("provider lock poisoned");
        if inner.active == name {
            bail!("cannot remove the active provider '{}'", name);
        }
        if inner.providers.remove(name).is_none() {
            bail!("provider '{}' not found", name);
        }
        Ok(())
    }

    /// List all providers with their active status.
    pub fn list(&self) -> Vec<ProviderEntry> {
        let inner = self.inner.read().expect("provider lock poisoned");
        inner
            .providers
            .keys()
            .map(|name| ProviderEntry {
                name: name.clone(),
                active: *name == inner.active,
            })
            .collect()
    }
}

impl std::fmt::Debug for ProviderManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read().expect("provider lock poisoned");
        f.debug_struct("ProviderManager")
            .field("active", &inner.active)
            .field("count", &inner.providers.len())
            .finish()
    }
}

impl Clone for ProviderManager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
