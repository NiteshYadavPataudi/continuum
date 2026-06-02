use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::{
    caps::{CallModels, Cap, ReadSecrets},
    ids::{ModelId, ProviderId},
    model::*,
    CancellationToken,
};

/// Google Gemini model provider using the Generative Language API.
pub struct GeminiProvider {
    api_key: String,
    api_url: String,
    client: reqwest::Client,
    models: Vec<ModelDescriptor>,
}

impl GeminiProvider {
    const DEFAULT_API_URL: &'static str = "https://generativelanguage.googleapis.com/v1";

    /// Create a Gemini provider with the given API key.
    pub fn new(api_key: String, _secrets: Cap<ReadSecrets>) -> Self {
        Self::with_base_url(api_key, Self::DEFAULT_API_URL.to_string())
    }

    /// Create a Gemini provider with a custom base URL.
    pub fn with_base_url(api_key: String, api_url: String) -> Self {
        use continuum_models_registry::MODELS;

        let prefix = "google/";
        let mut models: Vec<ModelDescriptor> = MODELS
            .entries()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, m)| {
                let model_id = k.trim_start_matches(prefix);
                ModelDescriptor::new(
                    ModelId::new(model_id),
                    m.name.to_string(),
                    m.context_window as u32,
                    m.tool_calls,
                    m.streaming,
                )
            })
            .collect();
        models.sort_by_key(|m| m.id.as_str().to_string());

        Self {
            api_key,
            api_url,
            client: reqwest::Client::new(),
            models,
        }
    }
}

#[async_trait]
impl ModelProvider for GeminiProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new("google")
    }

    fn models(&self) -> &[ModelDescriptor] {
        &self.models
    }

    async fn complete(
        &self,
        _cap: &Cap<CallModels>,
        req: CompletionRequest,
        cancel: CancellationToken,
    ) -> Result<CompletionStream, ModelError> {
        // Map messages to Gemini's content format.
        let contents: Vec<serde_json::Value> = req
            .messages
            .iter()
            .map(|m| {
                let role = match m.role.as_str() {
                    "assistant" => "model",
                    other => other,
                };
                serde_json::json!({
                    "role": role,
                    "parts": [{ "text": m.content }]
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": req.max_tokens.unwrap_or(8192),
            }
        });

        if let Some(temp) = req.temperature {
            body["generationConfig"]["temperature"] = serde_json::json!(temp);
        }

        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse",
            self.api_url,
            req.model.as_str(),
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ModelError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let provider = ProviderId::new("google");
            return Err(match status.as_u16() {
                429 => ModelError::RateLimited { provider },
                401 | 403 => ModelError::AuthFailed(provider),
                _ => {
                    let body = response.text().await.unwrap_or_default();
                    ModelError::Other(format!("Gemini HTTP {status}: {body}"))
                }
            });
        }

        let sse_stream = crate::sse::parse_sse(response, cancel);
        let delta_stream = sse_stream.filter_map(|result| async move {
            match result {
                Ok(val) => match delta_from_gemini_sse(&val) {
                    Ok(Some(d)) => Some(Ok(d)),
                    Ok(None) => None,
                    Err(e) => Some(Err(e)),
                },
                Err(e) => Some(Err(e)),
            }
        });

        Ok(delta_stream.boxed())
    }

    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse, ModelError> {
        let response = self
            .client
            .post(format!(
                "{}/models/{}:batchEmbedContents",
                self.api_url,
                req.model.as_str(),
            ))
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.api_key)
            .json(&serde_json::json!({
                "requests": req.inputs.iter().map(|text| serde_json::json!({
                    "model": format!("models/{}", req.model.as_str()),
                    "content": { "parts": [{ "text": text }] }
                })).collect::<Vec<_>>(),
            }))
            .send()
            .await
            .map_err(|e| ModelError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let provider = ProviderId::new("google");
            return Err(match status.as_u16() {
                429 => ModelError::RateLimited { provider },
                401 | 403 => ModelError::AuthFailed(provider),
                _ => {
                    let body = response.text().await.unwrap_or_default();
                    ModelError::Other(format!("Gemini HTTP {status}: {body}"))
                }
            });
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ModelError::Malformed(e.to_string()))?;

        let vectors: Vec<Vec<f32>> = data
            .get("embeddings")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| {
                        e.get("values")
                            .and_then(|v| v.as_array())
                            .and_then(|vals| {
                                vals.iter()
                                    .map(|v| v.as_f64().map(|f| f as f32))
                                    .collect::<Option<Vec<_>>>()
                            })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(EmbedResponse::new(vectors))
    }

    fn estimate_cost(&self, req: &CompletionRequest) -> CostEstimate {
        let input_tokens = req.messages.iter().map(|m| m.content.len() as u32 / 4).sum();
        let output_tokens = req.max_tokens.unwrap_or(8192);
        crate::cost::estimate_cost(self.id().as_str(), req.model.as_str(), input_tokens, output_tokens)
    }
}

fn delta_from_gemini_sse(val: &serde_json::Value) -> Result<Option<Delta>, ModelError> {
    // Gemini SSE: { "candidates": [{ "content": { "parts": [{ "text": "..." }] } }] }
    if let Some(candidates) = val.get("candidates").and_then(|v| v.as_array()) {
        if let Some(candidate) = candidates.first() {
            if let Some(text) = candidate
                .pointer("/content/parts/0/text")
                .and_then(|v| v.as_str())
            {
                if !text.is_empty() {
                    return Ok(Some(Delta::new(text.to_string(), false)));
                }
            }
            // Check finish reason
            if let Some("STOP" | "MAX_TOKENS") =
                candidate.get("finishReason").and_then(|v| v.as_str())
            {
                return Ok(Some(Delta::new(String::new(), true)));
            }
        }
    }
    Ok(None)
}
