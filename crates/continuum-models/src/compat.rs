use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::{
    caps::{CallModels, Cap, ReadSecrets},
    ids::{ModelId, ProviderId},
    model::*,
    CancellationToken,
};

/// Generic OpenAI-compatible provider.
///
/// Supports any API that speaks the OpenAI Chat Completions format:
/// DeepSeek, Groq, Mistral, Together, Fireworks, etc.
pub struct CompatProvider {
    provider_id: String,
    api_key: String,
    api_url: String,
    client: reqwest::Client,
    models: Vec<ModelDescriptor>,
}

impl CompatProvider {
    /// Create a provider for any OpenAI-compatible API.
    pub fn new(
        provider_id: impl Into<String>,
        api_key: String,
        api_url: impl Into<String>,
        _secrets: Cap<ReadSecrets>,
    ) -> Self {
        let provider_id = provider_id.into();
        let api_url = api_url.into();
        use continuum_models_registry::MODELS;

        let prefix = format!("{provider_id}/");
        let mut models: Vec<ModelDescriptor> = MODELS
            .entries()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, m)| {
                let model_id = k.trim_start_matches(&prefix);
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
            provider_id,
            api_key,
            api_url,
            client: reqwest::Client::new(),
            models,
        }
    }

    /// Convenience constructor for DeepSeek.
    pub fn deepseek(api_key: String, secrets: Cap<ReadSecrets>) -> Self {
        Self::new("deepseek", api_key, "https://api.deepseek.com/v1", secrets)
    }

    /// Convenience constructor for Groq.
    pub fn groq(api_key: String, secrets: Cap<ReadSecrets>) -> Self {
        Self::new("groq", api_key, "https://api.groq.com/openai/v1", secrets)
    }

    /// Convenience constructor for Mistral.
    pub fn mistral(api_key: String, secrets: Cap<ReadSecrets>) -> Self {
        Self::new("mistral", api_key, "https://api.mistral.ai/v1", secrets)
    }

    /// Convenience constructor for Together AI.
    pub fn together(api_key: String, secrets: Cap<ReadSecrets>) -> Self {
        Self::new("together", api_key, "https://api.together.xyz/v1", secrets)
    }

    /// Convenience constructor for Fireworks AI.
    pub fn fireworks(api_key: String, secrets: Cap<ReadSecrets>) -> Self {
        Self::new(
            "fireworks",
            api_key,
            "https://api.fireworks.ai/inference/v1",
            secrets,
        )
    }

    /// Convenience constructor for OpenRouter.
    pub fn openrouter(api_key: String, secrets: Cap<ReadSecrets>) -> Self {
        Self::new(
            "openrouter",
            api_key,
            "https://openrouter.ai/api/v1",
            secrets,
        )
    }
}

#[async_trait]
impl ModelProvider for CompatProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new(&self.provider_id)
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
            let provider = self.id();
            return Err(match status.as_u16() {
                429 => ModelError::RateLimited { provider },
                401 | 403 => ModelError::AuthFailed(provider),
                _ => {
                    let body = response.text().await.unwrap_or_default();
                    ModelError::Other(format!("{} HTTP {status}: {body}", self.provider_id))
                }
            });
        }

        let sse_stream = crate::sse::parse_sse(response, cancel);
        let delta_stream = sse_stream.filter_map(|result| async move {
            match result {
                Ok(val) => match delta_from_openai_sse(&val) {
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
            .post(self.api_url.replace("/chat/completions", "/embeddings"))
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
            let provider = self.id();
            return Err(match status.as_u16() {
                429 => ModelError::RateLimited { provider },
                401 | 403 => ModelError::AuthFailed(provider),
                _ => {
                    let body = response.text().await.unwrap_or_default();
                    ModelError::Other(format!("{} HTTP {status}: {body}", self.provider_id))
                }
            });
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
