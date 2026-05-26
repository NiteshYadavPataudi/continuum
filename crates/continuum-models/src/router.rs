use async_trait::async_trait;
use continuum_core::{
    ids::{ModelId, ProviderId},
    model::{ModelError, ModelIntent, ModelRouter, RoutedModel},
};

/// Cost-aware router that selects the cheapest suitable model.
/// Falls back from Ollama (free) → Haiku (cheap) → Sonnet (full).
pub struct CostAwareRouter;

#[async_trait]
impl ModelRouter for CostAwareRouter {
    async fn select(&self, intent: ModelIntent) -> Result<RoutedModel, ModelError> {
        let provider = ProviderId::new(continuum_models_registry::ANTHROPIC);

        match intent.budget_usd {
            Some(b) if b < 0.001 => {
                // Draft tier: prefer Ollama, fall back to Haiku
                Ok(RoutedModel::new(
                    ProviderId::new("ollama"),
                    ModelId::new("llama3.1"),
                    vec![(provider, ModelId::new("claude-haiku-3-5-20241022"))],
                ))
            }
            _ => {
                // Standard tier: Sonnet 4, fall back to Haiku
                Ok(RoutedModel::new(
                    provider.clone(),
                    ModelId::new("claude-sonnet-4-20250514"),
                    vec![(provider, ModelId::new("claude-haiku-3-5-20241022"))],
                ))
            }
        }
    }
}
