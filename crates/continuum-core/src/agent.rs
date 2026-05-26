//! [`Agent`] trait and supporting types.
//!
//! Each subagent in `continuum-agents` (planner-agent, coding-agent, etc.)
//! implements [`Agent`]. The runtime dispatches a task to the agent registered
//! for its [`AgentKind`].

use crate::{
    ids::{AgentId, TaskId},
    CancellationToken,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Discriminator for the eight built-in agent kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentKind {
    /// Creates execution DAGs from goals.
    Planner,
    /// Validates system design and architectural constraints.
    Architecture,
    /// Implements and refactors code.
    Coding,
    /// Generates and runs tests.
    Testing,
    /// Implements and audits security controls.
    Security,
    /// Reviews changes for maintainability and optimization.
    Review,
    /// Summarizes and compresses memory.
    Memory,
    /// Drives replay and crash recovery.
    Recovery,
}

/// Declarative capability flags an agent advertises to the scheduler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Whether the agent supports running in parallel with siblings.
    pub parallel_safe: bool,
    /// Whether the agent requires a sandbox.
    pub needs_sandbox: bool,
    /// Whether the agent needs network access through the sandbox.
    pub needs_network: bool,
    /// Maximum concurrency the agent should be scheduled with.
    pub max_concurrency: u32,
}

/// A unit of work dispatched to an agent. Concrete payload depends on
/// [`AgentKind`]; the runtime serializes the variant chosen by the planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AgentTask {
    /// The task node ID within the parent execution plan.
    pub task_id: TaskId,
    /// Opaque payload — the agent deserializes from a JSON value to whatever
    /// shape it expects. Kept opaque here so `continuum-core` stays free of
    /// per-agent payload schemas.
    pub payload: serde_json::Value,
}

impl AgentTask {
    /// Create a new agent task.
    pub fn new(task_id: TaskId, payload: serde_json::Value) -> Self {
        Self { task_id, payload }
    }
}

/// Bundle of handles an agent receives at execution time. The runtime
/// constructs this per task so agents never wire their own dependencies.
///
/// This is intentionally a placeholder during phase 1; concrete fields land
/// in phase 4 when the first agent is implemented.
#[derive(Debug)]
#[non_exhaustive]
pub struct AgentContext {
    /// The agent's stable ID.
    pub agent_id: AgentId,
}

impl AgentContext {
    /// Create a new agent context with a fresh agent ID.
    pub fn new() -> Self {
        Self {
            agent_id: AgentId::new(),
        }
    }
}

impl Default for AgentContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Outcome of an agent's [`Agent::handle`] call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AgentOutcome {
    /// Free-form payload describing what the agent produced. The runtime
    /// passes this to validation and recovery for journaling.
    pub artifacts: serde_json::Value,
}

impl AgentOutcome {
    /// Create a new outcome with the given artifacts.
    pub fn new(artifacts: serde_json::Value) -> Self {
        Self { artifacts }
    }
}

/// Errors specific to agent execution.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AgentError {
    /// Agent execution exceeded the budget (tokens, time, or cost).
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
    /// The agent was cancelled by the runtime.
    #[error("cancelled")]
    Cancelled,
    /// Free-form failure.
    #[error("agent failure: {0}")]
    Other(String),
}

/// Contract every subagent implements.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Stable identifier for this agent instance.
    fn id(&self) -> AgentId;

    /// Which built-in role this agent fills.
    fn kind(&self) -> AgentKind;

    /// Declared capability flags consulted by the scheduler.
    fn capabilities(&self) -> AgentCapabilities;

    /// Execute a task. Implementations must respect `cancel` and return
    /// [`AgentError::Cancelled`] promptly when it fires.
    async fn handle(
        &self,
        task: AgentTask,
        ctx: &AgentContext,
        cancel: CancellationToken,
    ) -> Result<AgentOutcome, AgentError>;
}
