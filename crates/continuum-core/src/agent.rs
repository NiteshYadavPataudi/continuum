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
use std::path::PathBuf;
use std::sync::Arc;
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

/// Snapshot of a task node in a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TaskSnapshot {
    /// Stable task identifier.
    pub task_id: TaskId,
    /// Human-readable label.
    pub label: String,
    /// Agent responsible for the task.
    pub agent: AgentKind,
    /// Upstream dependencies.
    pub depends_on: Vec<TaskId>,
}

impl TaskSnapshot {
    /// Create a new task snapshot.
    pub fn new(
        task_id: TaskId,
        label: impl Into<String>,
        agent: AgentKind,
        depends_on: Vec<TaskId>,
    ) -> Self {
        Self {
            task_id,
            label: label.into(),
            agent,
            depends_on,
        }
    }
}

/// Sink for live execution events.
pub trait ExecutionEventSink: Send + Sync {
    /// Publish the full task graph before execution starts.
    fn plan_loaded(&self, tasks: Vec<TaskSnapshot>);
    /// A task entered the ready queue.
    fn task_queued(
        &self,
        task_id: TaskId,
        agent: AgentKind,
        label: String,
        ready_group: usize,
        depends_on: Vec<TaskId>,
    );
    /// A task started running.
    fn task_started(
        &self,
        task_id: TaskId,
        agent: AgentKind,
        label: String,
        workspace: Option<PathBuf>,
    );
    /// A task emitted live progress.
    fn task_progress(
        &self,
        task_id: TaskId,
        agent: AgentKind,
        message: String,
        percent: Option<u8>,
    );
    /// A task is blocked.
    fn task_blocked(&self, task_id: TaskId, agent: AgentKind, reason: String);
    /// A task completed successfully.
    fn task_completed(
        &self,
        task_id: TaskId,
        agent: AgentKind,
        status: String,
        percent: Option<u8>,
    );
    /// A task failed.
    fn task_failed(&self, task_id: TaskId, agent: AgentKind, error: String);
    /// A workspace has been prepared for isolated execution.
    fn workspace_prepared(
        &self,
        task_id: TaskId,
        agent: AgentKind,
        workspace: PathBuf,
        isolated: bool,
        source: Option<PathBuf>,
    );
    /// A potential merge conflict was detected.
    fn conflict_detected(&self, task_id: TaskId, files: Vec<PathBuf>, reason: String);
    /// Merge has started for a task workspace.
    fn merge_started(&self, task_id: TaskId, workspace: PathBuf);
    /// Merge has completed for a task workspace.
    fn merge_completed(&self, task_id: TaskId, workspace: PathBuf, merged_files: usize);
    /// A tool started inside a task.
    fn tool_started(&self, task_id: TaskId, tool: String, message: String, percent: Option<u8>);
    /// A tool emitted live progress.
    fn tool_progress(&self, task_id: TaskId, tool: String, message: String, percent: Option<u8>);
    /// A tool completed.
    fn tool_completed(&self, task_id: TaskId, tool: String, message: String);
    /// A tool failed.
    fn tool_failed(&self, task_id: TaskId, tool: String, error: String);
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
#[derive(Clone)]
#[non_exhaustive]
pub struct AgentContext {
    /// The agent's stable ID.
    pub agent_id: AgentId,
    /// Optional task ID being processed.
    pub task_id: Option<TaskId>,
    /// Optional live event sink.
    pub reporter: Option<Arc<dyn ExecutionEventSink>>,
    /// Optional isolated workspace path for this task.
    pub workspace: Option<PathBuf>,
}

impl std::fmt::Debug for AgentContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentContext")
            .field("agent_id", &self.agent_id)
            .field("task_id", &self.task_id)
            .field("workspace", &self.workspace)
            .finish_non_exhaustive()
    }
}

impl AgentContext {
    /// Create a new agent context with a fresh agent ID.
    pub fn new() -> Self {
        Self {
            agent_id: AgentId::new(),
            task_id: None,
            reporter: None,
            workspace: None,
        }
    }

    /// Attach a live event sink.
    pub fn with_reporter(mut self, reporter: Arc<dyn ExecutionEventSink>) -> Self {
        self.reporter = Some(reporter);
        self
    }

    /// Attach the task identifier.
    pub fn with_task_id(mut self, task_id: TaskId) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// Attach the workspace path.
    pub fn with_workspace(mut self, workspace: PathBuf) -> Self {
        self.workspace = Some(workspace);
        self
    }

    /// Forward a task progress event if a reporter is attached.
    pub fn task_progress(&self, agent: AgentKind, message: impl Into<String>, percent: Option<u8>) {
        if let (Some(reporter), Some(task_id)) = (&self.reporter, self.task_id) {
            reporter.task_progress(task_id, agent, message.into(), percent);
        }
    }

    /// Forward a tool progress event if a reporter is attached.
    pub fn tool_progress(
        &self,
        tool: impl Into<String>,
        message: impl Into<String>,
        percent: Option<u8>,
    ) {
        if let (Some(reporter), Some(task_id)) = (&self.reporter, self.task_id) {
            reporter.tool_progress(task_id, tool.into(), message.into(), percent);
        }
    }

    /// Forward a task completion event.
    pub fn task_completed(&self, agent: AgentKind, status: impl Into<String>) {
        if let (Some(reporter), Some(task_id)) = (&self.reporter, self.task_id) {
            reporter.task_completed(task_id, agent, status.into(), None);
        }
    }

    /// Forward a task failure event.
    pub fn task_failed(&self, agent: AgentKind, error: impl Into<String>) {
        if let (Some(reporter), Some(task_id)) = (&self.reporter, self.task_id) {
            reporter.task_failed(task_id, agent, error.into());
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
