//! Layered configuration for Continuum.
//!
//! Resolution order (highest wins):
//! 1. CLI flags (handled by clap)
//! 2. `CONTINUUM_<PROVIDER>_API_KEY` environment variables
//! 3. Provider-native env vars (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, …)
//! 4. `~/.continuum/config.toml`

#![warn(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level Continuum configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Per-provider configuration (API keys, base URL overrides).
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

/// Per-provider settings stored in the config file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// API key for this provider.
    pub api_key: Option<String>,
    /// Override the provider's default base URL (useful for proxies or local deployments).
    pub base_url: Option<String>,
}

impl Config {
    /// Load config from `~/.continuum/config.toml`. Returns empty config on any error.
    pub fn load() -> Self {
        if let Some(path) = config_path() {
            if path.exists() {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Ok(cfg) = toml::from_str::<Config>(&text) {
                        return cfg;
                    }
                }
            }
        }
        Config::default()
    }

    /// Persist config to `~/.continuum/config.toml`.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = config_path().ok_or(ConfigError::NoHomeDir)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::Io(e.to_string()))?;
        }
        let text = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;
        std::fs::write(&path, text).map_err(|e| ConfigError::Io(e.to_string()))?;
        Ok(())
    }

    /// Resolve the API key for `provider`.
    ///
    /// Lookup order:
    /// 1. `CONTINUUM_<PROVIDER>_API_KEY` env var
    /// 2. `env_var_hint` (provider-native env var, e.g. `ANTHROPIC_API_KEY`)
    /// 3. `[providers.<provider>].api_key` in the config file
    pub fn api_key(&self, provider: &str, env_var_hint: &str) -> Option<String> {
        let continuum_var = format!(
            "CONTINUUM_{}_API_KEY",
            provider.to_uppercase().replace('-', "_")
        );
        if let Ok(v) = std::env::var(&continuum_var) {
            if !v.is_empty() {
                return Some(v);
            }
        }
        if !env_var_hint.is_empty() {
            if let Ok(v) = std::env::var(env_var_hint) {
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
        self.providers
            .get(provider)
            .and_then(|p| p.api_key.clone())
    }

    /// Resolve the base URL override for `provider`. `None` means use the built-in default.
    pub fn base_url(&self, provider: &str) -> Option<String> {
        let env_var = format!(
            "CONTINUUM_{}_BASE_URL",
            provider.to_uppercase().replace('-', "_")
        );
        if let Ok(v) = std::env::var(&env_var) {
            if !v.is_empty() {
                return Some(v);
            }
        }
        self.providers
            .get(provider)
            .and_then(|p| p.base_url.clone())
    }

    /// Set an API key for `provider` in memory. Call [`save`](Self::save) to persist.
    pub fn set_api_key(&mut self, provider: &str, key: &str) {
        self.providers
            .entry(provider.to_string())
            .or_default()
            .api_key = Some(key.to_string());
    }

    /// Set a base URL override for `provider` in memory.
    pub fn set_base_url(&mut self, provider: &str, url: &str) {
        self.providers
            .entry(provider.to_string())
            .or_default()
            .base_url = Some(url.to_string());
    }

    /// Clear a specific field (`api_key` or `base_url`) for `provider`.
    pub fn unset(&mut self, provider: &str, field: &str) {
        if let Some(p) = self.providers.get_mut(provider) {
            match field {
                "api_key" => p.api_key = None,
                "base_url" => p.base_url = None,
                _ => {}
            }
        }
    }
}

/// Filesystem path for the user-level config file (`~/.continuum/config.toml`).
pub fn config_path() -> Option<PathBuf> {
    directories::UserDirs::new()
        .map(|u| u.home_dir().join(".continuum").join("config.toml"))
}

/// Errors produced by config I/O operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Home directory could not be determined.
    #[error("could not determine home directory")]
    NoHomeDir,
    /// Filesystem I/O failure.
    #[error("I/O error: {0}")]
    Io(String),
    /// TOML serialization failure.
    #[error("serialize error: {0}")]
    Serialize(String),
}
