#![doc = "Planning engine: analyzes repo, decomposes goals, estimates cost, and produces execution contracts."]
#![warn(missing_docs)]

mod analyze;
mod contract;
mod estimate;
mod plan;

use async_trait::async_trait;
use continuum_core::ids::ModelId;
use continuum_core::model::ModelProvider;
use continuum_core::planner::*;
use continuum_core::repo::RepoIndex;
use std::sync::Arc;

pub use analyze::analyze;
pub use contract::contract;
pub use estimate::estimate;
pub use plan::plan;

/// Implementation of the [`Planner`] trait.
///
/// Pass a `ModelProvider` for LLM-driven DAG decomposition; omit it to use
/// the keyword-heuristic fallback.
pub struct PlanningEngine {
    model: Option<Arc<dyn ModelProvider>>,
    model_id: ModelId,
}

impl std::fmt::Debug for PlanningEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlanningEngine")
            .field("model_id", &self.model_id)
            .field("model", &self.model.as_ref().map(|_| "<provider>"))
            .finish()
    }
}

impl PlanningEngine {
    /// Create a planning engine with an optional model provider.
    pub fn new(model: Option<Arc<dyn ModelProvider>>, model_id: ModelId) -> Self {
        Self { model, model_id }
    }
}

impl Default for PlanningEngine {
    fn default() -> Self {
        Self::new(None, ModelId::from("claude-sonnet-4-5-20241022"))
    }
}

#[async_trait]
impl Planner for PlanningEngine {
    async fn analyze(
        &self,
        repo: Arc<dyn RepoIndex>,
        docs: &EngineeringDocs,
    ) -> Result<RepoAnalysis, PlanError> {
        analyze::analyze(repo, docs).await
    }

    async fn plan(&self, goal: Goal, analysis: &RepoAnalysis) -> Result<ExecutionPlan, PlanError> {
        plan::plan(goal, analysis, self.model.clone(), self.model_id.clone()).await
    }

    async fn estimate(&self, execution_plan: &ExecutionPlan) -> Result<PlanEstimate, PlanError> {
        estimate::estimate(execution_plan).await
    }

    async fn contract(
        &self,
        execution_plan: &ExecutionPlan,
    ) -> Result<ExecutionContract, PlanError> {
        let est = self.estimate(execution_plan).await?;
        contract::contract(execution_plan, &est).await
    }
}
