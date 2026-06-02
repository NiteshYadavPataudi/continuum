use async_trait::async_trait;
use continuum_core::{
    caps::{CallModels, Cap, ReadSecrets},
    ids::{ModelId, ProviderId},
    model::*,
    CancellationToken,
};
use futures::StreamExt;

/// Anthropic model provider using the Messages API.
pub struct AnthropicProvider {
    api_key: String,
    api_url: String,
    client: reqwest::Client,
    models: Vec<ModelDescriptor>,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with the given API key.
    pub fn new(api_key: String, _secrets: Cap<ReadSecrets>) -> Self {
        use continuum_models_registry::MODELS;

        // Load models from the registry snapshot instead of hardcoding them.
        let mut models: Vec<ModelDescriptor> = MODELS
            .entries()
            .filter(|(k, _)| k.starts_with("anthropic/"))
            .map(|(k, m)| {
                let model_id = k.trim_start_matches("anthropic/");
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
            api_url: "https://api.anthropic.com/v1/messages".to_string(),
            client: reqwest::Client::new(),
            models,
        }
    }
}

#[async_trait]
impl ModelProvider for AnthropicProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new(continuum_models_registry::ANTHROPIC)
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
        let url = self.api_url.clone();
        let messages: Vec<serde_json::Value> = req
            .messages
            .iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();

        let mut body = serde_json::json!({
            "model": req.model.as_str(),
            "messages": messages,
            "stream": true,
            "max_tokens": req.max_tokens.unwrap_or(4096),
        });

        if let Some(temp) = req.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ModelError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(handle_error_response(response, status).await);
        }

        let sse_stream = crate::sse::parse_sse(response, cancel);
        let delta_stream = sse_stream.filter_map(|result| async move {
            match result {
                Ok(val) => match delta_from_sse(&val) {
                    Ok(Some(delta)) => Some(Ok(delta)),
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
            .post(format!("{}/embeddings", self.api_url.trim_end_matches("/messages")))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": req.model.as_str(),
                "input": req.inputs,
            }))
            .send()
            .await
            .map_err(|e| ModelError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let provider = ProviderId::new(continuum_models_registry::ANTHROPIC);
            return Err(match status.as_u16() {
                429 => ModelError::RateLimited { provider },
                401 => ModelError::AuthFailed(provider),
                _ => {
                    let body = response.text().await.unwrap_or_default();
                    ModelError::Other(format!("Anthropic HTTP {}: {}", status, body))
                }
            });
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ModelError::Malformed(e.to_string()))?;

        let vectors = data
            .pointer("/data/0/values")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter()
                    .map(|v| v.as_f64().map(|f| f as f32))
                    .collect::<Option<Vec<_>>>()
            })
            .map(|vec| vec![vec])
            .unwrap_or_default();

        Ok(EmbedResponse::new(vectors))
    }

    fn estimate_cost(&self, req: &CompletionRequest) -> CostEstimate {
        let input_tokens = req.messages.iter().map(|m| m.content.len() as u32 / 4).sum();
        let output_tokens = req.max_tokens.unwrap_or(4096);
        crate::cost::estimate_cost(self.id().as_str(), req.model.as_str(), input_tokens, output_tokens)
    }
}

fn delta_from_sse(val: &serde_json::Value) -> Result<Option<Delta>, ModelError> {
    let event_type = val
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ModelError::Malformed("missing type in SSE data".into()))?;

    match event_type {
        "content_block_delta" => {
            let text = val
                .pointer("/delta/text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(Some(Delta::new(text.to_string(), false)))
        }
        "message_stop" | "content_block_stop" => Ok(Some(Delta::new(String::new(), true))),
        "error" => {
            let msg = val
                .pointer("/error/message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown anthropic error");
            Err(ModelError::Other(msg.to_string()))
        }
        _ => Ok(None),
    }
}

async fn handle_error_response(
    response: reqwest::Response,
    status: reqwest::StatusCode,
) -> ModelError {
    let provider = ProviderId::new(continuum_models_registry::ANTHROPIC);
    match status.as_u16() {
        429 => ModelError::RateLimited { provider },
        401 => ModelError::AuthFailed(provider),
        _ => {
            let body = response.text().await.unwrap_or_default();
            ModelError::Other(format!("HTTP {}: {}", status, body))
        }
    }
}
