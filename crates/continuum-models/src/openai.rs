use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::{
    caps::{CallModels, Cap, ReadSecrets},
    ids::{ModelId, ProviderId},
    model::*,
    CancellationToken,
};

/// OpenAI model provider using the Chat Completions API.
pub struct OpenAIProvider {
    api_key: String,
    api_url: String,
    client: reqwest::Client,
    models: Vec<ModelDescriptor>,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider.
    pub fn new(api_key: String, _secrets: Cap<ReadSecrets>) -> Self {
        Self::with_base_url(api_key, "https://api.openai.com/v1".to_string())
    }

    /// Create an OpenAI provider with a custom base URL (useful for proxies).
    pub fn with_base_url(api_key: String, api_url: String) -> Self {
        use continuum_models_registry::MODELS;

        let mut models: Vec<ModelDescriptor> = MODELS
            .entries()
            .filter(|(k, _)| k.starts_with("openai/"))
            .map(|(k, m)| {
                let model_id = k.trim_start_matches("openai/");
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
impl ModelProvider for OpenAIProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new("openai")
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
            .post(format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ModelError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(handle_openai_error(response, status).await);
        }

        let sse_stream = crate::sse::parse_sse(response, cancel);
        let delta_stream = sse_stream.filter_map(|result| async move {
            match result {
                Ok(val) => match delta_from_openai_sse(&val) {
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
            .post(format!("{}/embeddings", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": req.model.as_str(),
                "input": req.inputs,
            }))
            .send()
            .await
            .map_err(|e| ModelError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(handle_openai_error(response, status).await);
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ModelError::Malformed(e.to_string()))?;

        let vectors = vec![data
            .pointer("/data/0/embedding")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter()
                    .map(|v| v.as_f64().map(|f| f as f32))
                    .collect::<Option<Vec<_>>>()
            })
            .unwrap_or_default()];

        Ok(EmbedResponse::new(vectors))
    }

    fn estimate_cost(&self, req: &CompletionRequest) -> CostEstimate {
        let input_tokens = req.messages.iter().map(|m| m.content.len() as u32 / 4).sum();
        let output_tokens = req.max_tokens.unwrap_or(4096);
        crate::cost::estimate_cost(self.id().as_str(), req.model.as_str(), input_tokens, output_tokens)
    }
}

fn delta_from_openai_sse(val: &serde_json::Value) -> Result<Option<Delta>, ModelError> {
    let choices = val.get("choices").and_then(|v| v.as_array());
    match choices.and_then(|c| c.first()) {
        Some(choice) => {
            let delta = choice.get("delta").and_then(|d| d.get("content"));
            match delta.and_then(|v| v.as_str()) {
                Some(text) if !text.is_empty() => Ok(Some(Delta::new(text.to_string(), false))),
                _ => {
                    let finish = choice.get("finish_reason").and_then(|v| v.as_str());
                    match finish {
                        Some("stop") | Some("length") => Ok(Some(Delta::new(String::new(), true))),
                        _ => Ok(None),
                    }
                }
            }
        }
        None => Ok(None),
    }
}

async fn handle_openai_error(
    response: reqwest::Response,
    status: reqwest::StatusCode,
) -> ModelError {
    let provider = ProviderId::new("openai");
    match status.as_u16() {
        429 => ModelError::RateLimited { provider },
        401 => ModelError::AuthFailed(provider),
        _ => {
            let body = response.text().await.unwrap_or_default();
            ModelError::Other(format!("OpenAI HTTP {}: {}", status, body))
        }
    }
}
