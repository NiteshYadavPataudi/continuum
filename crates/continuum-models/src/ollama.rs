use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::{
    caps::{CallModels, Cap},
    ids::{ModelId, ProviderId},
    model::*,
    CancellationToken,
};

/// Ollama provider for local models. Useful for cheap-tier / offline work.
pub struct OllamaProvider {
    api_url: String,
    client: reqwest::Client,
    models: Vec<ModelDescriptor>,
    available: Vec<String>,
}

impl OllamaProvider {
    /// Create a new Ollama provider.
    pub fn new(api_url: Option<String>) -> Self {
        let url = api_url.unwrap_or_else(|| "http://localhost:11434".to_string());
        Self {
            models: vec![
                ModelDescriptor::new(
                    ModelId::new("llama3.1"),
                    "Llama 3.1".to_string(),
                    128_000,
                    true,
                    false,
                ),
                ModelDescriptor::new(
                    ModelId::new("mistral"),
                    "Mistral".to_string(),
                    32_000,
                    true,
                    false,
                ),
                ModelDescriptor::new(
                    ModelId::new("codellama"),
                    "CodeLlama".to_string(),
                    16_000,
                    true,
                    false,
                ),
            ],
            api_url: url,
            client: reqwest::Client::new(),
            available: Vec::new(),
        }
    }

    /// Check which models are available on the Ollama server.
    pub async fn refresh_available(&mut self) {
        let url = format!("{}/api/tags", self.api_url);
        if let Ok(resp) = self.client.get(&url).send().await {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(models) = data.get("models").and_then(|v| v.as_array()) {
                    self.available = models
                        .iter()
                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                        .map(|s| s.to_string())
                        .collect();
                }
            }
        }
    }
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new("ollama")
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

        let body = serde_json::json!({
            "model": req.model.as_str(),
            "messages": messages,
            "stream": true,
            "options": {
                "num_predict": req.max_tokens.unwrap_or(4096),
                "temperature": req.temperature.unwrap_or(0.7),
            },
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.api_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| ModelError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ModelError::Other(format!(
                "Ollama HTTP {}: {}",
                status,
                response.text().await.unwrap_or_default()
            )));
        }

        let sse_stream = crate::sse::parse_sse(response, cancel);
        let delta_stream = sse_stream.filter_map(|result| async move {
            match result {
                Ok(val) => match delta_from_ollama(&val) {
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
            .post(format!("{}/api/embeddings", self.api_url))
            .json(&serde_json::json!({
                "model": req.model.as_str(),
                "prompt": req.inputs.first().unwrap_or(&String::new()),
            }))
            .send()
            .await
            .map_err(|e| ModelError::Network(e.to_string()))?;

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ModelError::Malformed(e.to_string()))?;

        let vectors = vec![data
            .get("embedding")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter()
                    .map(|v| v.as_f64().map(|f| f as f32))
                    .collect::<Option<Vec<_>>>()
            })
            .unwrap_or_default()];

        Ok(EmbedResponse::new(vectors))
    }

    fn estimate_cost(&self, _req: &CompletionRequest) -> CostEstimate {
        CostEstimate::new(0, 0, 0.0)
    }
}

fn delta_from_ollama(val: &serde_json::Value) -> Result<Option<Delta>, ModelError> {
    if val.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Ok(Some(Delta::new(String::new(), true)));
    }

    let text = val
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Delta::new(text.to_string(), false)))
    }
}
