use continuum_core::ids::ProviderId;
use continuum_core::model::ModelError;

/// Convert a `reqwest` transport failure into a structured `ModelError`.
pub fn map_reqwest_error(err: reqwest::Error) -> ModelError {
    if err.is_timeout() {
        ModelError::Timeout
    } else {
        ModelError::Network(err.to_string())
    }
}

/// Convert an HTTP status and body into a structured `ModelError`.
pub fn map_http_error(
    provider: impl Into<ProviderId>,
    status: reqwest::StatusCode,
    body: String,
) -> ModelError {
    let provider = provider.into();
    match status.as_u16() {
        401 | 403 => ModelError::AuthFailed(provider),
        429 => ModelError::RateLimited { provider },
        500..=599 => ModelError::ServerError {
            provider,
            status: status.as_u16(),
            body,
        },
        _ => ModelError::Other(format!("{provider} HTTP {status}: {body}")),
    }
}
