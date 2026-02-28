//! `ProviderManager` — concurrent-safe named provider map with active-provider
//! swapping (DD#65, DD#67).

use crate::{Provider, ProviderConfig, build_provider};
use anyhow::{Result, bail};
use async_stream::try_stream;
use compact_str::CompactString;
use futures_core::Stream;
use futures_util::StreamExt;
use llm::{General, LLM, Message, Response, StreamChunk};
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
    /// Named provider instances with their configs.
    providers: BTreeMap<CompactString, (ProviderConfig, Provider)>,
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
    /// Create a new manager from a named map of provider configs.
    ///
    /// The first key (BTreeMap alphabetical order) becomes the active provider.
    /// Returns an error if the map is empty, any config fails validation, or
    /// any provider fails to build.
    pub async fn from_configs(configs: &BTreeMap<CompactString, ProviderConfig>) -> Result<Self> {
        if configs.is_empty() {
            bail!("at least one provider config is required");
        }

        let client = llm::Client::new();
        let mut providers = BTreeMap::new();

        for (name, config) in configs {
            config.validate()?;
            let provider = build_provider(config, client.clone()).await?;
            providers.insert(name.clone(), (config.clone(), provider));
        }

        let active = providers
            .keys()
            .next()
            .expect("non-empty checked above")
            .clone();

        Ok(Self {
            inner: Arc::new(RwLock::new(Inner {
                providers,
                active,
                client,
            })),
        })
    }

    /// Create a manager with a single provider.
    pub fn single(name: CompactString, config: ProviderConfig, provider: Provider) -> Self {
        let mut providers = BTreeMap::new();
        providers.insert(name.clone(), (config, provider));
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
        inner.providers[&inner.active].1.clone()
    }

    /// Get the name of the active provider.
    pub fn active_name(&self) -> CompactString {
        let inner = self.inner.read().expect("provider lock poisoned");
        inner.active.clone()
    }

    /// Get the model identifier of the active provider.
    pub fn active_model(&self) -> CompactString {
        let inner = self.inner.read().expect("provider lock poisoned");
        inner.providers[&inner.active].0.model.clone()
    }

    /// Get a clone of the active provider's config.
    pub fn active_config(&self) -> ProviderConfig {
        let inner = self.inner.read().expect("provider lock poisoned");
        inner.providers[&inner.active].0.clone()
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

    /// Add a new provider. Validates config first. Replaces any existing
    /// provider with the same name.
    pub async fn add(&self, name: &str, config: &ProviderConfig) -> Result<()> {
        config.validate()?;
        let client = {
            let inner = self.inner.read().expect("provider lock poisoned");
            inner.client.clone()
        };
        let provider = build_provider(config, client).await?;
        let mut inner = self.inner.write().expect("provider lock poisoned");
        inner
            .providers
            .insert(CompactString::from(name), (config.clone(), provider));
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

impl LLM for ProviderManager {
    type ChatConfig = General;

    async fn send(&self, config: &General, messages: &[Message]) -> Result<Response> {
        self.active().send(config, messages).await
    }

    fn stream(
        &self,
        config: General,
        messages: &[Message],
        usage: bool,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send {
        let provider = self.active();
        let messages = messages.to_vec();
        try_stream! {
            let mut stream = std::pin::pin!(provider.stream(config, &messages, usage));
            while let Some(chunk) = stream.next().await {
                yield chunk?;
            }
        }
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
