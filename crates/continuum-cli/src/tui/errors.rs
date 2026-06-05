use continuum_config::Config;
use continuum_core::model::ModelError;
use continuum_models_registry::PROVIDERS;

fn provider_hint(provider_id: &str) -> (String, String, String) {
    let continuum_env = format!(
        "CONTINUUM_{}_API_KEY",
        provider_id.to_uppercase().replace('-', "_")
    );
    let config_key = format!("[providers.{provider_id}].api_key");
    let native_env = PROVIDERS
        .get(provider_id)
        .map(|meta| meta.env_var.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    (continuum_env, native_env, config_key)
}

pub fn missing_api_key_message(provider_id: &str) -> String {
    let (continuum_env, native_env, config_key) = provider_hint(provider_id);
    let mut lines = vec![format!(
        "No API key is configured for provider '{provider_id}'."
    )];

    if !native_env.is_empty() {
        lines.push(format!(
            "Set the env var `{native_env}` or `{continuum_env}`."
        ));
    } else {
        lines.push(format!("Set the env var `{continuum_env}`."));
    }

    lines.push(format!(
        "Or save it in `~/.continuum/config.toml` as `{config_key}`."
    ));
    lines.push(format!(
        "Examples: `continuum login` or `continuum config set {provider_id}.api_key <key>`"
    ));
    lines.join(" ")
}

pub fn model_error_message(provider_id: &str, err: &ModelError, config: &Config) -> String {
    let provider_label = provider_id.to_string();
    let api_key_hint = missing_api_key_message(provider_id);
    let saved_key = match PROVIDERS.get(provider_id) {
        Some(meta) => config.api_key(provider_id, meta.env_var).is_some(),
        None => false,
    };

    match err {
        ModelError::AuthFailed(_) => {
            if saved_key {
                format!(
                    "The API key for '{provider_label}' was rejected. Check the key, then try again. {api_key_hint}"
                )
            } else {
                api_key_hint
            }
        }
        ModelError::RateLimited { .. } => format!(
            "Provider '{provider_label}' rate-limited this request. Please wait a moment and try again, or switch models."
        ),
        ModelError::Timeout => format!(
            "The request to '{provider_label}' timed out. Check your network or try again with a smaller prompt."
        ),
        ModelError::Network(message) => format!(
            "A network error occurred while talking to '{provider_label}': {message}"
        ),
        ModelError::ServerError {
            provider,
            status,
            body,
        } => format!(
            "Provider '{}' returned an error ({status}): {}",
            provider.as_str(),
            body.trim()
        ),
        ModelError::Malformed(message) => format!(
            "The provider '{provider_label}' returned a malformed response: {message}"
        ),
        ModelError::ContextExceeded { model } => format!(
            "The selected model '{model}' cannot handle this prompt because the context window was exceeded."
        ),
        ModelError::Cancelled => "The request was cancelled.".to_string(),
        ModelError::Other(message) => format!(
            "The provider '{provider_label}' returned an unexpected error: {message}"
        ),
        _ => format!(
            "The provider '{provider_label}' returned an unsupported model error: {err}"
        ),
    }
}

pub fn startup_notice(provider_id: &str, config: &Config) -> Option<String> {
    let meta = PROVIDERS.get(provider_id)?;
    if meta.env_var.is_empty() {
        return None;
    }

    if config.api_key(provider_id, meta.env_var).is_some() {
        None
    } else {
        Some(missing_api_key_message(provider_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use continuum_core::ids::ProviderId;

    #[test]
    fn missing_api_key_message_mentions_examples() {
        let msg = missing_api_key_message("openrouter");
        assert!(msg.contains("CONTINUUM_OPENROUTER_API_KEY"));
        assert!(msg.contains("continuum login"));
        assert!(msg.contains("continuum config set openrouter.api_key"));
    }

    #[test]
    fn model_error_message_maps_auth_failure() {
        let cfg = Config::default();
        let msg = model_error_message(
            "openrouter",
            &ModelError::AuthFailed(ProviderId::new("openrouter")),
            &cfg,
        );
        assert!(msg.contains("openrouter"));
        assert!(msg.contains("CONTINUUM_OPENROUTER_API_KEY"));
    }
}
