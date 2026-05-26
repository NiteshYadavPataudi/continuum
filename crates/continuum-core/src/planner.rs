//! Planner trait and execution-plan types.
//!
//! `continuum-planner` builds an [`ExecutionPlan`] (a DAG of [`TaskNode`]s)
//! from a [`Goal`] and the engineering docs, then estimates token/runtime/cost
//! before the runtime asks for user approval via an [`ExecutionContract`].

use crate::{agent::AgentKind, ids::TaskId, repo::RepoIndex};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// A user-supplied goal that drives planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Goal {
    /// Natural-language description of the desired outcome.
    pub prompt: String,
    /// Optional structured constraints (e.g. budget, deadlines).
    pub constraints: serde_json::Value,
}

impl Goal {
    /// Create a new goal with the given prompt and no constraints.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            constraints: serde_json::Value::Null,
        }
    }
}

/// Parsed view of the eight engineering docs the runtime consumes at startup.
/// Concrete schemas live in `continuum-markdown`; phase 1 keeps this opaque.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EngineeringDocs {
    /// Raw parsed content keyed by canonical doc name (`VISION`, `PRODUCT`, ...).
    pub docs: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Output of [`Planner::analyze`] — what the planner learned from the repo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RepoAnalysis {
    /// Human-readable summary.
    pub summary: String,
    /// Detected services / entry points.
    pub services: Vec<String>,
}

/// One node in an [`ExecutionPlan`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TaskNode {
    /// Stable ID within the plan.
    pub id: TaskId,
    /// Which agent should handle this node.
    pub agent_kind: AgentKind,
    /// Free-form payload passed to the agent.
    pub payload: serde_json::Value,
    /// Brief description, used in approval UI and live monitor.
    pub label: String,
}

/// Edge type in the DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dependency {
    /// Strict happens-before.
    HappensBefore,
    /// Soft hint that the parent's artifacts inform the child.
    Informs,
}

/// Full plan: forward DAG + rollback DAG.
///
/// Phase 1 keeps the graph opaque (just a Vec of nodes + Vec of edges). When
/// the planner is implemented in phase 3, this can be promoted to a real
/// `petgraph::DiGraph<TaskNode, Dependency>` without breaking the trait.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExecutionPlan {
    /// Task nodes (ordering is topologically sortable via `edges`).
    pub nodes: Vec<TaskNode>,
    /// Edges in the forward DAG.
    pub edges: Vec<(TaskId, TaskId, Dependency)>,
    /// Edges in the rollback DAG.
    pub rollback_edges: Vec<(TaskId, TaskId)>,
}

/// Token / runtime / cost estimate for an [`ExecutionPlan`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PlanEstimate {
    /// Estimated total input tokens across all nodes.
    pub input_tokens: u64,
    /// Estimated total output tokens.
    pub output_tokens: u64,
    /// Estimated wall-clock seconds.
    pub runtime_secs: u64,
    /// Estimated USD cost.
    pub usd: f64,
    /// Risk score in `0.0..=1.0`.
    pub risk: f32,
}

/// Human-approvable contract describing what the plan will do.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExecutionContract {
    /// Plan being approved.
    pub plan: ExecutionPlan,
    /// Estimate at the time the contract was drafted.
    pub estimate: PlanEstimate,
    /// Bullet-point summary of the user-visible changes.
    pub summary: Vec<String>,
}

/// Errors specific to planning.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PlanError {
    /// Could not understand the goal.
    #[error("invalid goal: {0}")]
    InvalidGoal(String),
    /// Could not analyse the repository.
    #[error("analysis failed: {0}")]
    AnalysisFailed(String),
    /// Model call failed during planning.
    #[error("model: {0}")]
    Model(String),
    /// Catch-all.
    #[error("planner error: {0}")]
    Other(String),
}

/// Planning engine contract.
#[async_trait]
pub trait Planner: Send + Sync {
    /// Analyse the target repository under the lens of the engineering docs.
    async fn analyze(
        &self,
        repo: Arc<dyn RepoIndex>,
        docs: &EngineeringDocs,
    ) -> Result<RepoAnalysis, PlanError>;

    /// Build an [`ExecutionPlan`] for the given goal.
    async fn plan(&self, goal: Goal, analysis: &RepoAnalysis) -> Result<ExecutionPlan, PlanError>;

    /// Estimate tokens/runtime/cost/risk for the plan.
    async fn estimate(&self, plan: &ExecutionPlan) -> Result<PlanEstimate, PlanError>;

    /// Produce a human-approvable contract.
    async fn contract(&self, plan: &ExecutionPlan) -> Result<ExecutionContract, PlanError>;
}
