//! `ProviderManager` — concurrent-safe named provider map with active-provider
//! swapping (DD#65, DD#67).

use crate::{Provider, ProviderConfig, build_provider};
use anyhow::{Result, bail};
use async_stream::try_stream;
use compact_str::CompactString;
use futures_core::Stream;
use futures_util::StreamExt;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use wcore::model::{General, LLM, Message, Response, StreamChunk};

/// Manages a set of named providers with an active selection.
///
/// All methods that read or mutate the inner state acquire the `RwLock`.
/// `active()` returns a clone of the current `Provider` — callers do not
/// hold the lock while performing LLM calls.
pub struct ProviderManager {
    inner: Arc<RwLock<Inner>>,
}

struct Inner {
    /// Provider instances keyed by model name.
    providers: BTreeMap<CompactString, (ProviderConfig, Provider)>,
    /// Model name of the currently active provider.
    active: CompactString,
    /// Shared HTTP client for constructing new providers.
    client: reqwest::Client,
}

/// Info about a single provider entry returned by `list()`.
#[derive(Debug, Clone)]
pub struct ProviderEntry {
    /// Provider model name (key).
    pub name: CompactString,
    /// Whether this is the active provider.
    pub active: bool,
}

impl ProviderManager {
    /// Create a new manager from a list of provider configs.
    ///
    /// The first element becomes the active provider.
    /// Returns an error if the slice is empty, any config fails validation, or
    /// any provider fails to build.
    pub async fn from_configs(configs: &[ProviderConfig]) -> Result<Self> {
        if configs.is_empty() {
            bail!("at least one provider config is required");
        }

        let client = reqwest::Client::new();
        let mut providers = BTreeMap::new();

        for config in configs {
            config.validate()?;
            let provider = build_provider(config, client.clone()).await?;
            providers.insert(config.model.clone(), (config.clone(), provider));
        }

        let active = configs[0].model.clone();

        Ok(Self {
            inner: Arc::new(RwLock::new(Inner {
                providers,
                active,
                client,
            })),
        })
    }

    /// Create a manager with a single provider.
    pub fn single(config: ProviderConfig, provider: Provider) -> Self {
        let model = config.model.clone();
        let mut providers = BTreeMap::new();
        providers.insert(model.clone(), (config, provider));
        Self {
            inner: Arc::new(RwLock::new(Inner {
                providers,
                active: model,
                client: reqwest::Client::new(),
            })),
        }
    }

    /// Get a clone of the active provider.
    pub fn active(&self) -> Provider {
        let inner = self.inner.read().expect("provider lock poisoned");
        inner.providers[&inner.active].1.clone()
    }

    /// Get the model name of the active provider (also its key).
    pub fn active_model(&self) -> CompactString {
        let inner = self.inner.read().expect("provider lock poisoned");
        inner.active.clone()
    }

    /// Get a clone of the active provider's config.
    pub fn active_config(&self) -> ProviderConfig {
        let inner = self.inner.read().expect("provider lock poisoned");
        inner.providers[&inner.active].0.clone()
    }

    /// Switch to a different provider by model name. Returns an error if the
    /// name is not found.
    pub fn switch(&self, model: &str) -> Result<()> {
        let mut inner = self.inner.write().expect("provider lock poisoned");
        if !inner.providers.contains_key(model) {
            bail!("provider '{}' not found", model);
        }
        inner.active = CompactString::from(model);
        Ok(())
    }

    /// Add a new provider. Validates config first. Replaces any existing
    /// provider with the same model name.
    pub async fn add(&self, config: &ProviderConfig) -> Result<()> {
        config.validate()?;
        let client = {
            let inner = self.inner.read().expect("provider lock poisoned");
            inner.client.clone()
        };
        let provider = build_provider(config, client).await?;
        let mut inner = self.inner.write().expect("provider lock poisoned");
        inner
            .providers
            .insert(config.model.clone(), (config.clone(), provider));
        Ok(())
    }

    /// Remove a provider by model name. Fails if the provider is currently
    /// active.
    pub fn remove(&self, model: &str) -> Result<()> {
        let mut inner = self.inner.write().expect("provider lock poisoned");
        if inner.active == model {
            bail!("cannot remove the active provider '{}'", model);
        }
        if inner.providers.remove(model).is_none() {
            bail!("provider '{}' not found", model);
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
