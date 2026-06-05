//! Model provider and router traits.
//!
//! `continuum-models` consumes `models.dev`-style provider metadata and
//! implements one [`ModelProvider`] per backend. A [`ModelRouter`] decides
//! which provider/model to invoke based on a [`ModelIntent`] (complexity,
//! latency target, cost budget).

use crate::{
    caps::{CallModels, Cap},
    ids::{ModelId, ProviderId},
    CancellationToken,
};
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Static metadata about a single model offered by a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModelDescriptor {
    /// Provider-qualified model ID.
    pub id: ModelId,
    /// Human-readable name.
    pub name: String,
    /// Maximum context window in tokens.
    pub context_window: u32,
    /// Whether the model supports tool calls.
    pub tool_calls: bool,
    /// Whether the model supports streaming.
    pub streaming: bool,
}

impl ModelDescriptor {
    /// Create a new model descriptor.
    pub fn new(
        id: ModelId,
        name: String,
        context_window: u32,
        tool_calls: bool,
        streaming: bool,
    ) -> Self {
        Self {
            id,
            name,
            context_window,
            tool_calls,
            streaming,
        }
    }
}

/// Generic message slot used by [`CompletionRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role: `"system"`, `"user"`, `"assistant"`, `"tool"`.
    pub role: String,
    /// Text content.
    pub content: String,
}

impl Message {
    /// Create a new message.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }
}

/// One streaming chunk from a completion call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Delta {
    /// Text fragment to append to the running response.
    pub text: String,
    /// True when this is the terminal delta.
    pub done: bool,
}

impl Delta {
    /// Create a new delta.
    pub fn new(text: String, done: bool) -> Self {
        Self { text, done }
    }
}

/// Stream of completion deltas. Boxed for dyn-compatibility.
pub type CompletionStream = BoxStream<'static, Result<Delta, ModelError>>;

/// Inputs to a completion call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CompletionRequest {
    /// Model to invoke (must match a `ModelDescriptor::id` from the same provider).
    pub model: ModelId,
    /// Conversation history.
    pub messages: Vec<Message>,
    /// Sampling temperature (0.0 = deterministic).
    pub temperature: Option<f32>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
}

impl CompletionRequest {
    /// Create a new completion request.
    pub fn new(model: ModelId, messages: Vec<Message>) -> Self {
        Self {
            model,
            messages,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Set temperature.
    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = Some(t);
        self
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = Some(n);
        self
    }
}

/// Embedding request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbedRequest {
    /// Embedding model to invoke.
    pub model: ModelId,
    /// Texts to embed.
    pub inputs: Vec<String>,
}

/// Embedding response payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbedResponse {
    /// One vector per input.
    pub vectors: Vec<Vec<f32>>,
}

impl EmbedResponse {
    /// Create a new embedding response.
    pub fn new(vectors: Vec<Vec<f32>>) -> Self {
        Self { vectors }
    }
}

/// Predicted cost of a request. All fields are best-effort estimates from
/// the provider's pricing table (sourced via `continuum-models-registry`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CostEstimate {
    /// Estimated input tokens.
    pub input_tokens: u32,
    /// Estimated output tokens.
    pub output_tokens: u32,
    /// Estimated USD cost.
    pub usd: f64,
}

impl CostEstimate {
    /// Create a new cost estimate.
    pub fn new(input_tokens: u32, output_tokens: u32, usd: f64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            usd,
        }
    }
}

/// What the caller is trying to accomplish — the router uses this to pick
/// the cheapest model that meets the constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModelIntent {
    /// Classifier of the task (e.g. `"code"`, `"plan"`, `"summarize"`).
    pub task_class: String,
    /// Subjective complexity: `"draft"`, `"standard"`, `"deep"`.
    pub complexity: String,
    /// Maximum USD the caller is willing to spend on this call.
    pub budget_usd: Option<f64>,
    /// Whether streaming is required.
    pub require_streaming: bool,
}

/// Result of routing: chosen primary plus fallback chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RoutedModel {
    /// Primary provider to call first.
    pub provider: ProviderId,
    /// Primary model.
    pub model: ModelId,
    /// Ordered fallback chain on rate-limit or transient error.
    pub fallbacks: Vec<(ProviderId, ModelId)>,
}

impl RoutedModel {
    /// Create a new routed model.
    pub fn new(
        provider: ProviderId,
        model: ModelId,
        fallbacks: Vec<(ProviderId, ModelId)>,
    ) -> Self {
        Self {
            provider,
            model,
            fallbacks,
        }
    }
}

/// Errors specific to model interaction.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ModelError {
    /// Rate limit hit — the router should fall back.
    #[error("rate limited by provider {provider}")]
    RateLimited {
        /// Provider that returned the rate-limit response.
        provider: ProviderId,
    },
    /// Provider authentication failed (missing or invalid API key).
    #[error("auth failed for provider {0}")]
    AuthFailed(ProviderId),
    /// The request would exceed the provider's context window.
    #[error("context window exceeded for {model}")]
    ContextExceeded {
        /// Model whose context was overflowed.
        model: ModelId,
    },
    /// The model returned an unexpected payload shape.
    #[error("malformed response: {0}")]
    Malformed(String),
    /// Network error.
    #[error("network error: {0}")]
    Network(String),
    /// The request timed out.
    #[error("timeout")]
    Timeout,
    /// The provider returned a server-side error.
    #[error("server error {status} from {provider}")]
    ServerError {
        /// Provider that returned the error.
        provider: ProviderId,
        /// HTTP status code.
        status: u16,
        /// Response body or summary.
        body: String,
    },
    /// The call was cancelled before completion.
    #[error("cancelled")]
    Cancelled,
    /// Catch-all.
    #[error("model error: {0}")]
    Other(String),
}

/// Contract every backend (OpenAI, Anthropic, Gemini, DeepSeek, Ollama, ...) implements.
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Stable provider identifier.
    fn id(&self) -> ProviderId;

    /// Static catalog of supported models.
    fn models(&self) -> &[ModelDescriptor];

    /// Run a (possibly streaming) completion.
    async fn complete(
        &self,
        _cap: &Cap<CallModels>,
        req: CompletionRequest,
        cancel: CancellationToken,
    ) -> Result<CompletionStream, ModelError>;

    /// Compute embeddings.
    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse, ModelError>;

    /// Best-effort cost estimate for the request.
    fn estimate_cost(&self, req: &CompletionRequest) -> CostEstimate;
}

/// Routing strategy. Implementations in `continuum-models` translate an
/// [`ModelIntent`] into a concrete [`RoutedModel`] using rules from
/// `MODEL_RULES.md` and live cost data.
#[async_trait]
pub trait ModelRouter: Send + Sync {
    /// Pick a model for the given intent.
    async fn select(&self, intent: ModelIntent) -> Result<RoutedModel, ModelError>;
}
