use crate::Provider;
use crabllm_core::{Error, GatewayConfig};
use rand::Rng;
use std::{collections::HashMap, time::Duration};

/// A provider entry with its routing weight and retry config.
#[derive(Debug, Clone)]
pub struct Deployment {
    pub provider: Provider,
    pub weight: u16,
    pub max_retries: u32,
    pub timeout: Duration,
}

/// Maps model names to weighted provider lists for routing.
#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, Vec<Deployment>>,
    aliases: HashMap<String, String>,
    /// Precomputed model name → provider name lookup (avoids per-request HashMap rebuild).
    model_providers: HashMap<String, String>,
}

impl ProviderRegistry {
    /// Create a registry directly from pre-built provider lists and aliases.
    pub fn new(
        providers: HashMap<String, Vec<Deployment>>,
        aliases: HashMap<String, String>,
        model_providers: HashMap<String, String>,
    ) -> Self {
        ProviderRegistry {
            providers,
            aliases,
            model_providers,
        }
    }

    /// Build the registry from gateway config.
    pub fn from_config(config: &GatewayConfig) -> Result<Self, Error> {
        for (provider_name, provider_config) in &config.providers {
            let p = Provider::from(provider_config);
            validate_provider(provider_name, provider_config, &p)?;
        }

        let mut providers: HashMap<String, Vec<Deployment>> = HashMap::new();

        for provider_config in config.providers.values() {
            let provider = Provider::from(provider_config);

            let deployment = Deployment {
                provider,
                weight: provider_config.weight.unwrap_or(1),
                max_retries: provider_config.max_retries.unwrap_or(2),
                timeout: Duration::from_secs(provider_config.timeout.unwrap_or(30)),
            };
            for model_name in &provider_config.models {
                providers
                    .entry(model_name.clone())
                    .or_default()
                    .push(deployment.clone());
            }
        }

        let mut model_providers = HashMap::new();
        for (provider_name, provider_config) in &config.providers {
            for model in &provider_config.models {
                model_providers.insert(model.clone(), provider_name.clone());
            }
        }

        Ok(Self::new(
            providers,
            config.aliases.clone(),
            model_providers,
        ))
    }

    /// Look up the provider name for a model. O(1) HashMap lookup.
    pub fn provider_name(&self, model: &str) -> Option<&str> {
        self.model_providers.get(model).map(|s| s.as_str())
    }

    /// Return all registered model names.
    pub fn model_names(&self) -> impl Iterator<Item = &str> {
        self.model_providers.keys().map(|s| s.as_str())
    }

    /// Resolve a model name through aliases. Returns the canonical name.
    pub fn resolve<'a>(&'a self, model: &'a str) -> &'a str {
        self.aliases.get(model).map(|s| s.as_str()).unwrap_or(model)
    }

    /// Select a provider for a model using weighted random selection.
    /// Returns None if the model is not registered.
    pub fn dispatch(&self, model: &str) -> Option<&Deployment> {
        let list = self.providers.get(model)?;
        if list.len() == 1 {
            return Some(&list[0]);
        }

        let total: u32 = list.iter().map(|d| d.weight as u32).sum();
        if total == 0 {
            return Some(&list[0]);
        }

        let mut pick = rand::rng().random_range(0..total);
        for d in list {
            let w = d.weight as u32;
            if pick < w {
                return Some(d);
            }
            pick -= w;
        }

        // Fallback (shouldn't happen).
        Some(&list[0])
    }

    /// Return all deployments for a model, ordered for fallback:
    /// selected provider first, then remaining sorted by descending weight.
    /// Returns None if the model is not registered.
    pub fn dispatch_list(&self, model: &str) -> Option<Vec<&Deployment>> {
        let list = self.providers.get(model)?;
        if list.len() == 1 {
            return Some(vec![&list[0]]);
        }

        let total: u32 = list.iter().map(|d| d.weight as u32).sum();
        let selected_idx = if total == 0 {
            0
        } else {
            let mut pick = rand::rng().random_range(0..total);
            let mut idx = 0;
            for (i, d) in list.iter().enumerate() {
                let w = d.weight as u32;
                if pick < w {
                    idx = i;
                    break;
                }
                pick -= w;
            }
            idx
        };

        // Build result: selected first, then remaining sorted by descending weight.
        let mut result = Vec::with_capacity(list.len());
        result.push(&list[selected_idx]);

        let mut remaining: Vec<(usize, &Deployment)> = list
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != selected_idx)
            .collect();
        remaining.sort_by(|a, b| b.1.weight.cmp(&a.1.weight));
        result.extend(remaining.into_iter().map(|(_, d)| d));

        Some(result)
    }

    /// Check if a model is registered (after alias resolution).
    pub fn has_model(&self, model: &str) -> bool {
        self.providers.contains_key(model)
    }
}

/// Validate provider-specific required fields.
fn validate_provider(
    name: &str,
    config: &crabllm_core::ProviderConfig,
    provider: &Provider,
) -> Result<(), Error> {
    match provider {
        Provider::Openai { base_url, .. } if base_url.is_empty() => Err(Error::Config(format!(
            "provider '{name}' ({:?}) requires a base_url",
            config.kind,
        ))),
        Provider::Anthropic { api_key } | Provider::Google { api_key } if api_key.is_empty() => {
            Err(Error::Config(format!(
                "provider '{name}' ({:?}) requires an api_key",
                config.kind,
            )))
        }
        Provider::Azure { api_key, .. } if api_key.is_empty() => Err(Error::Config(format!(
            "provider '{name}' (azure) requires an api_key",
        ))),
        Provider::Bedrock {
            region,
            access_key,
            secret_key,
        } => {
            if region.is_empty() {
                return Err(Error::Config(format!(
                    "provider '{name}' (bedrock) requires a region",
                )));
            }
            if access_key.is_empty() {
                return Err(Error::Config(format!(
                    "provider '{name}' (bedrock) requires an access_key",
                )));
            }
            if secret_key.is_empty() {
                return Err(Error::Config(format!(
                    "provider '{name}' (bedrock) requires a secret_key",
                )));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
