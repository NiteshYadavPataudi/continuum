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
        404 => ModelError::ServerError {
            provider,
            status: status.as_u16(),
            body,
        },
        500..=599 => ModelError::ServerError {
            provider,
            status: status.as_u16(),
            body,
        },
        _ => ModelError::Other(format!("{provider} HTTP {status}: {body}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_404_to_server_error() {
        let err = map_http_error(
            ProviderId::new("openrouter"),
            reqwest::StatusCode::NOT_FOUND,
            "No endpoints found".to_string(),
        );

        match err {
            ModelError::ServerError {
                provider,
                status,
                body,
            } => {
                assert_eq!(provider.as_str(), "openrouter");
                assert_eq!(status, 404);
                assert!(body.contains("No endpoints found"));
            }
            other => panic!("unexpected error mapping: {other}"),
        }
    }
}
