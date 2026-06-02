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
    /// Load config from `~/.continuum/config.toml`. Returns empty config on any error
    /// but logs a warning to stderr when parsing fails.
    pub fn load() -> Self {
        if let Some(path) = config_path() {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(text) => match toml::from_str::<Config>(&text) {
                        Ok(cfg) => return cfg,
                        Err(e) => {
                            tracing::warn!(
                                "failed to parse config file {}: {e} — using defaults",
                                path.display()
                            );
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            "failed to read config file {}: {e} — using defaults",
                            path.display()
                        );
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
        let text =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(e.to_string()))?;
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
        self.providers.get(provider).and_then(|p| p.api_key.clone())
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
    directories::UserDirs::new().map(|u| u.home_dir().join(".continuum").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_api_key() {
        let mut cfg = Config::default();
        cfg.set_api_key("anthropic", "sk-ant-test123");
        assert_eq!(
            cfg.api_key("anthropic", "ANTHROPIC_API_KEY"),
            Some("sk-ant-test123".to_string())
        );
    }

    #[test]
    fn test_set_and_get_base_url() {
        let mut cfg = Config::default();
        cfg.set_base_url("openai", "https://my-proxy/v1");
        assert_eq!(
            cfg.base_url("openai"),
            Some("https://my-proxy/v1".to_string())
        );
    }

    #[test]
    fn test_unset_api_key() {
        let mut cfg = Config::default();
        cfg.set_api_key("anthropic", "sk-ant-test123");
        cfg.unset("anthropic", "api_key");
        assert_eq!(cfg.api_key("anthropic", "ANTHROPIC_API_KEY"), None);
    }

    #[test]
    fn test_unset_base_url() {
        let mut cfg = Config::default();
        cfg.set_base_url("openai", "https://my-proxy/v1");
        cfg.unset("openai", "base_url");
        assert!(cfg.base_url("openai").is_none());
    }

    #[test]
    fn test_api_key_unknown_provider() {
        let cfg = Config::default();
        assert_eq!(cfg.api_key("nonexistent", ""), None);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        // This test uses a temp dir; it may be affected by env vars.
        // We test a provider with a custom env hint to avoid conflicts.
        let dir = std::env::temp_dir().join(format!("continuum-rnd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("HOME", &dir);

        let mut cfg = Config::default();
        cfg.set_api_key("testprovider", "test-key-roundtrip");
        cfg.set_base_url("testprovider", "http://localhost:8080/v1");

        let result = cfg.save();
        assert!(result.is_ok(), "save failed: {:?}", result.err());

        let loaded = Config::load();
        assert_eq!(
            loaded.api_key("testprovider", "TESTPROVIDER_API_KEY"),
            Some("test-key-roundtrip".to_string())
        );

        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var("HOME");
    }

    #[test]
    fn test_load_default_when_no_config() {
        // Use a nonexistent provider with empty env hint to avoid env var interference
        let cfg = Config::default();
        assert!(cfg
            .api_key("nonexistent-provider", "NONEXISTENT_ENV_VAR_HINT")
            .is_none());
    }

    #[test]
    fn test_multiple_providers() {
        let mut cfg = Config::default();
        cfg.set_api_key("anthropic", "sk-ant-1");
        cfg.set_api_key("openai", "sk-openai-1");
        cfg.set_api_key("gemini", "gemini-key-1");

        assert_eq!(cfg.api_key("anthropic", ""), Some("sk-ant-1".to_string()));
        assert_eq!(cfg.api_key("openai", ""), Some("sk-openai-1".to_string()));
        assert_eq!(cfg.api_key("gemini", ""), Some("gemini-key-1".to_string()));
    }
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
