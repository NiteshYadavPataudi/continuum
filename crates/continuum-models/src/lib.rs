#![warn(missing_docs)]

//! Model provider implementations and routing.
//!
//! Each provider is feature-gated so you only compile what you need.
//! The `compat` module provides a generic OpenAI-compatible provider for
//! DeepSeek, Groq, Mistral, Together, Fireworks, and similar services.

#[cfg(feature = "anthropic")]
mod anthropic;
mod compat;
mod cost;
#[cfg(feature = "gemini")]
mod gemini;
#[cfg(feature = "ollama")]
mod ollama;
#[cfg(feature = "openai")]
mod openai;
mod router;
mod sse;

#[cfg(feature = "anthropic")]
pub use anthropic::AnthropicProvider;
pub use compat::CompatProvider;
#[cfg(feature = "gemini")]
pub use gemini::GeminiProvider;
#[cfg(feature = "ollama")]
pub use ollama::OllamaProvider;
#[cfg(feature = "openai")]
pub use openai::OpenAIProvider;
pub use router::CostAwareRouter;

use std::sync::Arc;
use continuum_core::caps::Cap;
use continuum_core::model::ModelProvider;

/// Dynamically instantiate the model provider based on provider ID, API key, and base URL.
pub fn load_provider(
    provider_id: &str,
    api_key: String,
    base_url_override: Option<String>,
) -> Arc<dyn ModelProvider> {
    let _secrets = Cap::grant();
    match provider_id {
        #[cfg(feature = "anthropic")]
        "anthropic" => Arc::new(AnthropicProvider::new(api_key, _secrets)),
        #[cfg(feature = "openai")]
        "openai" => {
            if let Some(base_url) = base_url_override {
                Arc::new(OpenAIProvider::with_base_url(api_key, base_url))
            } else {
                Arc::new(OpenAIProvider::new(api_key, _secrets))
            }
        }
        #[cfg(feature = "gemini")]
        "google" | "gemini" => Arc::new(GeminiProvider::new(api_key, _secrets)),
        #[cfg(feature = "ollama")]
        "ollama" => Arc::new(OllamaProvider::new(base_url_override)),
        // Fallback to CompatProvider for all other OpenAI-compatible endpoints (including openrouter)
        _ => {
            let default_base_url = continuum_models_registry::PROVIDERS
                .get(provider_id)
                .map(|p| p.api_base_url.to_string())
                .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
            let base_url = base_url_override.unwrap_or(default_base_url);
            Arc::new(CompatProvider::new(provider_id, api_key, base_url, _secrets))
        }
    }
}

/// Load provider using settings in the configuration.
///
/// Returns `None` when the config does not contain an API key for the provider.
/// Emits a `tracing::warn` message to help users diagnose missing configuration.
pub fn load_from_config(
    config: &continuum_config::Config,
    provider_id: &str,
) -> Option<Arc<dyn ModelProvider>> {
    let is_local = matches!(
        provider_id,
        "ollama" | "lmstudio" | "privatemode-ai" | "localhost"
    );
    let api_key = if is_local {
        String::new()
    } else {
        let meta = continuum_models_registry::PROVIDERS.get(provider_id);
        match meta {
            Some(meta) => match config.api_key(provider_id, meta.env_var) {
                Some(key) => key,
                None => {
                    tracing::warn!(
                        "no API key configured for provider '{}'. Set via env var {} or config file.",
                        provider_id,
                        meta.env_var
                    );
                    return None;
                }
            },
            None => {
                tracing::warn!("unknown provider '{}' — not found in registry", provider_id);
                return None;
            }
        }
    };
    let base_url = config.base_url(provider_id);
    Some(load_provider(provider_id, api_key, base_url))
}

