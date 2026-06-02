use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tokio::sync::broadcast;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use continuum_core::agent::{
    Agent, AgentContext, AgentKind, AgentOutcome, AgentTask, ExecutionEventSink, TaskSnapshot,
};
use continuum_core::memory::{MemoryItem, MemoryLayer};
use continuum_core::model::ModelProvider;
use continuum_core::planner::{ExecutionPlan, PlanError};
use continuum_core::recovery::SessionState;
use continuum_core::CancellationToken;
use continuum_telemetry::DashboardEvent;

use crate::session::Session;

/// Walks an ExecutionPlan DAG, dispatching each node to the appropriate Agent.
pub struct Scheduler {
    agents: HashMap<AgentKind, Arc<dyn Agent>>,
    concurrency: Arc<Semaphore>,
    agent_limits: HashMap<AgentKind, Arc<Semaphore>>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    /// Create a scheduler with real agents (where dependencies are available).
    pub fn new() -> Self {
        let mut agents: HashMap<AgentKind, Arc<dyn Agent>> = HashMap::new();
        let mut agent_limits: HashMap<AgentKind, Arc<Semaphore>> = HashMap::new();
        for kind in [
            AgentKind::Planner,
            AgentKind::Architecture,
            AgentKind::Coding,
            AgentKind::Testing,
            AgentKind::Security,
            AgentKind::Review,
            AgentKind::Memory,
            AgentKind::Recovery,
        ] {
            let agent: Arc<dyn Agent> = Arc::new(continuum_agents::StubAgent::new(kind));
            agent_limits.insert(
                kind,
                Arc::new(Semaphore::new(
                    agent.capabilities().max_concurrency.max(1) as usize
                )),
            );
            agents.insert(kind, agent);
        }
        Self {
            agents,
            concurrency: Arc::new(Semaphore::new(4)),
            agent_limits,
        }
    }

    /// Create a scheduler injecting concrete agent instances.
    pub fn with_agents(agent_map: HashMap<AgentKind, Arc<dyn Agent>>) -> Self {
        let mut agent_limits = HashMap::new();
        for (kind, agent) in &agent_map {
            let max = agent.capabilities().max_concurrency.max(1) as usize;
            agent_limits.insert(*kind, Arc::new(Semaphore::new(max)));
        }
        Self {
            agents: agent_map,
            concurrency: Arc::new(Semaphore::new(4)),
            agent_limits,
        }
    }

    /// Create a scheduler with real agents wired to model providers.
    /// Uses `ModelId::new("default")` as the model identifier.
    /// If `memory_store` is `None`, a stub `MemoryAgent` is used.
    pub fn with_models(
        model: Option<Arc<dyn ModelProvider>>,
        memory_store: Option<Arc<dyn continuum_core::memory::MemoryStore>>,
    ) -> Self {
        use continuum_agents::*;
        let model_id = continuum_core::ids::ModelId::new("default");
        let mut agents: HashMap<AgentKind, Arc<dyn Agent>> = HashMap::new();
        agents.insert(
            AgentKind::Planner,
            Arc::new(PlannerAgent::new(model.clone(), model_id.clone())),
        );
        agents.insert(
            AgentKind::Architecture,
            Arc::new(ArchitectureAgent::new(model.clone(), model_id.clone())),
        );
        agents.insert(
            AgentKind::Coding,
            Arc::new(CodingAgent::new(model.clone(), model_id.clone())),
        );
        agents.insert(
            AgentKind::Testing,
            Arc::new(TestingAgent::new(model.clone(), model_id.clone())),
        );
        agents.insert(
            AgentKind::Security,
            Arc::new(SecurityAgent::new(model.clone(), model_id.clone())),
        );
        agents.insert(
            AgentKind::Review,
            Arc::new(ReviewAgent::new(model.clone(), model_id.clone())),
        );
        agents.insert(
            AgentKind::Recovery,
            Arc::new(RecoveryAgent::new(model.clone(), model_id.clone())),
        );
        let memory_agent: Arc<dyn Agent> = match memory_store {
            Some(store) => Arc::new(MemoryAgent::new(store, model, model_id, 64000)),
            None => Arc::new(StubAgent::new(AgentKind::Memory)),
        };
        agents.insert(AgentKind::Memory, memory_agent);
        let mut agent_limits = HashMap::new();
        for (kind, agent) in &agents {
            let max = agent.capabilities().max_concurrency.max(1) as usize;
            agent_limits.insert(*kind, Arc::new(Semaphore::new(max)));
        }
        Self {
            agents,
            concurrency: Arc::new(Semaphore::new(4)),
            agent_limits,
        }
    }

    /// Register a custom agent, overriding the default stub for its kind.
    pub fn register(&mut self, agent: Arc<dyn Agent>) {
        let kind = agent.kind();
        let max = agent.capabilities().max_concurrency.max(1) as usize;
        self.agents.insert(kind, agent);
        self.agent_limits
            .insert(kind, Arc::new(Semaphore::new(max)));
    }

    /// Execute the plan within a session context.
    pub async fn run_with_session(
        &self,
        plan: &ExecutionPlan,
        session: &Session,
        cancel: CancellationToken,
    ) -> Result<Vec<AgentOutcome>, PlanError> {
        self.run_with_session_events(plan, session, cancel, None)
            .await
    }

    /// Execute the plan within a session context and stream live events.
    pub async fn run_with_session_events(
        &self,
        plan: &ExecutionPlan,
        session: &Session,
        cancel: CancellationToken,
        event_tx: Option<broadcast::Sender<DashboardEvent>>,
    ) -> Result<Vec<AgentOutcome>, PlanError> {
        let order = topological_sort(plan)?;
        let reporter = event_tx
            .as_ref()
            .map(|tx| Arc::new(BroadcastEventSink::new(tx.clone())) as Arc<dyn ExecutionEventSink>);
        let node_index_by_id: HashMap<_, _> = plan
            .nodes
            .iter()
            .enumerate()
            .map(|(idx, node)| (node.id, idx))
            .collect();

        let mut indegree = vec![0usize; plan.nodes.len()];
        let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); plan.nodes.len()];
        for (from, to, _dep) in &plan.edges {
            let from_idx = *node_index_by_id
                .get(from)
                .ok_or_else(|| PlanError::Other(format!("edge source {from} not in plan nodes")))?;
            let to_idx = *node_index_by_id
                .get(to)
                .ok_or_else(|| PlanError::Other(format!("edge target {to} not in plan nodes")))?;
            outgoing[from_idx].push(to_idx);
            indegree[to_idx] = indegree[to_idx].saturating_add(1);
        }

        // Validate the DAG before executing.
        if order.len() != plan.nodes.len() {
            return Err(PlanError::Other(
                "cycle detected in execution plan DAG".into(),
            ));
        }

        if let Some(reporter) = reporter.as_ref() {
            reporter.plan_loaded(
                plan.nodes
                    .iter()
                    .map(|node| {
                        let depends_on = plan
                            .edges
                            .iter()
                            .filter_map(
                                |(from, to, _)| {
                                    if *to == node.id {
                                        Some(*from)
                                    } else {
                                        None
                                    }
                                },
                            )
                            .collect();
                        TaskSnapshot::new(node.id, node.label.clone(), node.agent_kind, depends_on)
                    })
                    .collect(),
            );
        }

        let total_nodes = plan.nodes.len();
        let completed_nodes = Arc::new(AtomicUsize::new(0));
        let base_root = session
            .workspace_root
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let mut ready: VecDeque<usize> = indegree
            .iter()
            .enumerate()
            .filter(|(_, &d)| d == 0)
            .map(|(i, _)| i)
            .collect();
        let mut outcomes_by_idx: Vec<Option<AgentOutcome>> = vec![None; plan.nodes.len()];
        let mut wave_index = 0usize;

        while !ready.is_empty() {
            if cancel.is_cancelled() {
                return Err(PlanError::Other("execution cancelled".into()));
            }

            let wave: Vec<usize> = ready.drain(..).collect();
            for &idx in &wave {
                let node = &plan.nodes[idx];
                if let Some(reporter) = reporter.as_ref() {
                    let depends_on = plan
                        .edges
                        .iter()
                        .filter_map(|(from, to, _)| if *to == node.id { Some(*from) } else { None })
                        .collect();
                    reporter.task_queued(
                        node.id,
                        node.agent_kind,
                        node.label.clone(),
                        wave_index,
                        depends_on,
                    );
                }
            }

            let mut parallel_wave = Vec::new();
            let mut serial_wave = Vec::new();
            for &idx in &wave {
                let node = &plan.nodes[idx];
                let agent = self.agents.get(&node.agent_kind).ok_or_else(|| {
                    PlanError::Other(format!("no agent for {:?}", node.agent_kind))
                })?;
                if agent.capabilities().parallel_safe {
                    parallel_wave.push(idx);
                } else {
                    serial_wave.push(idx);
                }
            }

            let mut joinset = JoinSet::new();
            for idx in parallel_wave {
                let node = plan.nodes[idx].clone();
                let agent = self.agents.get(&node.agent_kind).cloned().ok_or_else(|| {
                    PlanError::Other(format!("no agent for {:?}", node.agent_kind))
                })?;
                let session_id = session.id;
                let memory = session.memory.clone();
                let recovery = session.recovery.clone();
                let reporter = reporter.clone();
                let cancel = cancel.child_token();
                let completed_nodes = completed_nodes.clone();
                let permit_limit = self
                    .agent_limits
                    .get(&node.agent_kind)
                    .cloned()
                    .unwrap_or_else(|| Arc::new(Semaphore::new(1)));
                let global_limit = self.concurrency.clone();
                let task_base_root = base_root.clone();

                joinset.spawn(async move {
                    let outcome = execute_task_node(
                        agent,
                        node,
                        AgentContext::new(),
                        session_id,
                        task_base_root,
                        memory,
                        recovery,
                        cancel,
                        reporter,
                        permit_limit,
                        global_limit,
                        completed_nodes,
                        total_nodes,
                    )
                    .await;
                    (idx, outcome)
                });
            }

            while let Some(joined) = joinset.join_next().await {
                let (idx, outcome) =
                    joined.map_err(|e| PlanError::Other(format!("task join failed: {e}")))?;
                match outcome {
                    Ok(outcome) => {
                        outcomes_by_idx[idx] = Some(outcome);
                    }
                    Err(err) => {
                        cancel.cancel();
                        return Err(err);
                    }
                }
            }

            for idx in serial_wave {
                if cancel.is_cancelled() {
                    return Err(PlanError::Other("execution cancelled".into()));
                }

                let node = plan.nodes[idx].clone();
                let agent = self.agents.get(&node.agent_kind).cloned().ok_or_else(|| {
                    PlanError::Other(format!("no agent for {:?}", node.agent_kind))
                })?;
                let memory = session.memory.clone();
                let recovery = session.recovery.clone();
                let reporter = reporter.clone();
                let cancel = cancel.child_token();
                let completed_nodes = completed_nodes.clone();
                let permit_limit = self
                    .agent_limits
                    .get(&node.agent_kind)
                    .cloned()
                    .unwrap_or_else(|| Arc::new(Semaphore::new(1)));
                let global_limit = self.concurrency.clone();
                let outcome = execute_task_node(
                    agent,
                    node,
                    AgentContext::new(),
                    session.id,
                    base_root.clone(),
                    memory,
                    recovery,
                    cancel,
                    reporter,
                    permit_limit,
                    global_limit,
                    completed_nodes,
                    total_nodes,
                )
                .await?;
                outcomes_by_idx[idx] = Some(outcome);
            }

            for idx in wave {
                for &child_idx in &outgoing[idx] {
                    indegree[child_idx] = indegree[child_idx].saturating_sub(1);
                    if indegree[child_idx] == 0 {
                        ready.push_back(child_idx);
                    }
                }
            }

            wave_index += 1;
        }

        let mut outcomes = Vec::with_capacity(plan.nodes.len());
        for outcome in outcomes_by_idx.into_iter().flatten() {
            outcomes.push(outcome);
        }
        Ok(outcomes)
    }

    /// Execute the plan without session context (backwards-compatible).
    pub async fn run(
        &self,
        plan: &ExecutionPlan,
        cancel: CancellationToken,
    ) -> Result<Vec<AgentOutcome>, PlanError> {
        let session = crate::session::Session::new();
        self.run_with_session(plan, &session, cancel).await
    }
}

struct NoopEventSink;

impl ExecutionEventSink for NoopEventSink {
    fn plan_loaded(&self, _tasks: Vec<TaskSnapshot>) {}
    fn task_queued(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _agent: AgentKind,
        _label: String,
        _ready_group: usize,
        _depends_on: Vec<continuum_core::ids::TaskId>,
    ) {
    }
    fn task_started(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _agent: AgentKind,
        _label: String,
        _workspace: Option<PathBuf>,
    ) {
    }
    fn task_progress(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _agent: AgentKind,
        _message: String,
        _percent: Option<u8>,
    ) {
    }
    fn task_blocked(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _agent: AgentKind,
        _reason: String,
    ) {
    }
    fn task_completed(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _agent: AgentKind,
        _status: String,
        _percent: Option<u8>,
    ) {
    }
    fn task_failed(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _agent: AgentKind,
        _error: String,
    ) {
    }
    fn workspace_prepared(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _agent: AgentKind,
        _workspace: PathBuf,
        _isolated: bool,
        _source: Option<PathBuf>,
    ) {
    }
    fn conflict_detected(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _files: Vec<PathBuf>,
        _reason: String,
    ) {
    }
    fn merge_started(&self, _task_id: continuum_core::ids::TaskId, _workspace: PathBuf) {}
    fn merge_completed(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _workspace: PathBuf,
        _merged_files: usize,
    ) {
    }
    fn tool_started(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _tool: String,
        _message: String,
        _percent: Option<u8>,
    ) {
    }
    fn tool_progress(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _tool: String,
        _message: String,
        _percent: Option<u8>,
    ) {
    }
    fn tool_completed(
        &self,
        _task_id: continuum_core::ids::TaskId,
        _tool: String,
        _message: String,
    ) {
    }
    fn tool_failed(&self, _task_id: continuum_core::ids::TaskId, _tool: String, _error: String) {}
}

struct BroadcastEventSink {
    tx: broadcast::Sender<DashboardEvent>,
}

impl BroadcastEventSink {
    fn new(tx: broadcast::Sender<DashboardEvent>) -> Self {
        Self { tx }
    }

    fn send(&self, event: DashboardEvent) {
        let _ = self.tx.send(event);
    }
}

impl ExecutionEventSink for BroadcastEventSink {
    fn plan_loaded(&self, tasks: Vec<TaskSnapshot>) {
        self.send(DashboardEvent::PlanLoaded { tasks });
    }

    fn task_queued(
        &self,
        task_id: continuum_core::ids::TaskId,
        agent: AgentKind,
        label: String,
        ready_group: usize,
        depends_on: Vec<continuum_core::ids::TaskId>,
    ) {
        self.send(DashboardEvent::TaskQueued {
            task_id: task_id.to_string(),
            agent: format!("{:?}", agent),
            label,
            ready_group,
            depends_on: depends_on.into_iter().map(|id| id.to_string()).collect(),
        });
    }

    fn task_started(
        &self,
        task_id: continuum_core::ids::TaskId,
        agent: AgentKind,
        label: String,
        workspace: Option<PathBuf>,
    ) {
        self.send(DashboardEvent::TaskStarted {
            task_id: task_id.to_string(),
            agent: format!("{:?}", agent),
            label,
            workspace: workspace.map(|p| p.display().to_string()),
        });
    }

    fn task_progress(
        &self,
        task_id: continuum_core::ids::TaskId,
        agent: AgentKind,
        message: String,
        percent: Option<u8>,
    ) {
        self.send(DashboardEvent::TaskProgress {
            task_id: task_id.to_string(),
            agent: format!("{:?}", agent),
            message,
            percent,
        });
    }

    fn task_blocked(&self, task_id: continuum_core::ids::TaskId, agent: AgentKind, reason: String) {
        self.send(DashboardEvent::TaskBlocked {
            task_id: task_id.to_string(),
            agent: format!("{:?}", agent),
            reason,
        });
    }

    fn task_completed(
        &self,
        task_id: continuum_core::ids::TaskId,
        agent: AgentKind,
        status: String,
        percent: Option<u8>,
    ) {
        self.send(DashboardEvent::TaskCompleted {
            task_id: task_id.to_string(),
            agent: format!("{:?}", agent),
            status,
            percent,
        });
    }

    fn task_failed(&self, task_id: continuum_core::ids::TaskId, agent: AgentKind, error: String) {
        self.send(DashboardEvent::TaskFailed {
            task_id: task_id.to_string(),
            agent: format!("{:?}", agent),
            error,
        });
    }

    fn workspace_prepared(
        &self,
        task_id: continuum_core::ids::TaskId,
        agent: AgentKind,
        workspace: PathBuf,
        isolated: bool,
        source: Option<PathBuf>,
    ) {
        self.send(DashboardEvent::WorkspacePrepared {
            task_id: task_id.to_string(),
            agent: format!("{:?}", agent),
            workspace: workspace.display().to_string(),
            isolated,
            source: source.map(|p| p.display().to_string()),
        });
    }

    fn conflict_detected(
        &self,
        task_id: continuum_core::ids::TaskId,
        files: Vec<PathBuf>,
        reason: String,
    ) {
        self.send(DashboardEvent::Merge {
            task_id: task_id.to_string(),
            workspace: files
                .first()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            merged_files: files.len(),
            conflict: Some(reason),
        });
    }

    fn merge_started(&self, task_id: continuum_core::ids::TaskId, workspace: PathBuf) {
        self.send(DashboardEvent::Log {
            level: "INFO".into(),
            target: "merge".into(),
            message: format!("merge started for {task_id} @ {}", workspace.display()),
        });
    }

    fn merge_completed(
        &self,
        task_id: continuum_core::ids::TaskId,
        workspace: PathBuf,
        merged_files: usize,
    ) {
        self.send(DashboardEvent::Merge {
            task_id: task_id.to_string(),
            workspace: workspace.display().to_string(),
            merged_files,
            conflict: None,
        });
    }

    fn tool_started(
        &self,
        task_id: continuum_core::ids::TaskId,
        tool: String,
        message: String,
        percent: Option<u8>,
    ) {
        self.send(DashboardEvent::Log {
            level: "INFO".into(),
            target: tool,
            message: format!("{task_id}: {message} ({percent:?})"),
        });
    }

    fn tool_progress(
        &self,
        task_id: continuum_core::ids::TaskId,
        tool: String,
        message: String,
        percent: Option<u8>,
    ) {
        self.send(DashboardEvent::Log {
            level: "INFO".into(),
            target: tool,
            message: format!("{task_id}: {message} ({percent:?})"),
        });
    }

    fn tool_completed(&self, task_id: continuum_core::ids::TaskId, tool: String, message: String) {
        self.send(DashboardEvent::Log {
            level: "INFO".into(),
            target: tool,
            message: format!("{task_id}: {message}"),
        });
    }

    fn tool_failed(&self, task_id: continuum_core::ids::TaskId, tool: String, error: String) {
        self.send(DashboardEvent::Log {
            level: "ERROR".into(),
            target: tool,
            message: format!("{task_id}: {error}"),
        });
    }
}

#[derive(Debug, Clone)]
struct WorkspaceLease {
    path: PathBuf,
    isolated: bool,
    source: Option<PathBuf>,
}

fn prepare_workspace(
    base_root: &Path,
    session_id: continuum_core::ids::SessionId,
    node: &continuum_core::planner::TaskNode,
    reporter: Option<&Arc<dyn ExecutionEventSink>>,
) -> Result<WorkspaceLease, PlanError> {
    let workspace = base_root
        .join(".continuum")
        .join("workspaces")
        .join(session_id.to_string())
        .join(node.id.to_string());
    if workspace.exists() {
        fs::remove_dir_all(&workspace)
            .map_err(|e| PlanError::Other(format!("cleanup workspace: {e}")))?;
    }
    fs::create_dir_all(&workspace)
        .map_err(|e| PlanError::Other(format!("create workspace: {e}")))?;

    let git_repo = is_git_repo(base_root);
    let mut isolated = false;
    let source = if git_repo {
        let result = Command::new("git")
            .args([
                "-C",
                &base_root.display().to_string(),
                "worktree",
                "add",
                "--detach",
                "--force",
                &workspace.display().to_string(),
                "HEAD",
            ])
            .output();
        if let Ok(output) = result {
            if output.status.success() {
                isolated = true;
                Some(base_root.to_path_buf())
            } else {
                copy_dir_filtered(base_root, &workspace)
                    .map_err(|e| PlanError::Other(format!("copy workspace: {e}")))?;
                None
            }
        } else {
            copy_dir_filtered(base_root, &workspace)
                .map_err(|e| PlanError::Other(format!("copy workspace: {e}")))?;
            None
        }
    } else {
        copy_dir_filtered(base_root, &workspace)
            .map_err(|e| PlanError::Other(format!("copy workspace: {e}")))?;
        None
    };

    if let Some(reporter) = reporter {
        reporter.workspace_prepared(
            node.id,
            node.agent_kind,
            workspace.clone(),
            isolated,
            source.clone(),
        );
    }

    Ok(WorkspaceLease {
        path: workspace,
        isolated,
        source,
    })
}

fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

fn copy_dir_filtered(from: &Path, to: &Path) -> std::io::Result<()> {
    if !to.exists() {
        fs::create_dir_all(to)?;
    }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy() == ".continuum" {
            continue;
        }
        let dest = to.join(name);
        if path.is_dir() {
            fs::create_dir_all(&dest)?;
            copy_dir_filtered(&path, &dest)?;
        } else if path.is_file() {
            fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}

fn write_task_artifacts(
    workspace: &Path,
    node: &continuum_core::planner::TaskNode,
    outcome: &AgentOutcome,
) -> Result<(), PlanError> {
    let target_path = node
        .payload
        .get("target_path")
        .and_then(|v| v.as_str())
        .or_else(|| node.payload.get("path").and_then(|v| v.as_str()));
    let code = outcome.artifacts.get("code").and_then(|v| v.as_str());
    if let (Some(target_path), Some(code)) = (target_path, code) {
        let mut path = workspace.to_path_buf();
        for component in target_path.split('/').chain(target_path.split('\\')) {
            match component {
                "" | "." => continue,
                ".." => {
                    return Err(PlanError::Other(
                        "refusing to write outside workspace".into(),
                    ));
                }
                part => path.push(part),
            }
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| PlanError::Other(format!("create artifact parent: {e}")))?;
        }
        fs::write(&path, code).map_err(|e| PlanError::Other(format!("write artifact: {e}")))?;
    }
    Ok(())
}

fn merge_workspace(
    base_root: &Path,
    workspace: &Path,
    task_id: continuum_core::ids::TaskId,
    _agent: AgentKind,
    reporter: Option<&Arc<dyn ExecutionEventSink>>,
) -> Result<Option<String>, PlanError> {
    if !is_git_repo(base_root) || !is_git_repo(workspace) {
        return Ok(None);
    }

    let patch = Command::new("git")
        .args([
            "-C",
            &workspace.display().to_string(),
            "diff",
            "--binary",
            "HEAD",
        ])
        .output()
        .map_err(|e| PlanError::Other(format!("git diff failed: {e}")))?;

    if patch.stdout.is_empty() {
        return Ok(None);
    }

    if let Some(reporter) = reporter {
        reporter.merge_started(task_id, workspace.to_path_buf());
    }

    let mut apply = Command::new("git")
        .args([
            "-C",
            &base_root.display().to_string(),
            "apply",
            "--3way",
            "--whitespace=nowarn",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| PlanError::Other(format!("git apply spawn failed: {e}")))?;

    if let Some(stdin) = apply.stdin.as_mut() {
        stdin
            .write_all(&patch.stdout)
            .map_err(|e| PlanError::Other(format!("git apply write failed: {e}")))?;
    }

    let output = apply
        .wait_with_output()
        .map_err(|e| PlanError::Other(format!("git apply wait failed: {e}")))?;

    let merged_files = Command::new("git")
        .args([
            "-C",
            &workspace.display().to_string(),
            "diff",
            "--name-only",
            "HEAD",
        ])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.lines().count())
        .unwrap_or(0);

    if output.status.success() {
        if let Some(reporter) = reporter {
            reporter.merge_completed(task_id, workspace.to_path_buf(), merged_files);
        }
        Ok(None)
    } else {
        let reason = String::from_utf8_lossy(&output.stderr).to_string();
        if let Some(reporter) = reporter {
            reporter.conflict_detected(task_id, vec![workspace.to_path_buf()], reason.clone());
        }
        Ok(Some(reason))
    }
}

#[allow(clippy::too_many_arguments)]
async fn execute_task_node(
    agent: Arc<dyn Agent>,
    node: continuum_core::planner::TaskNode,
    ctx: AgentContext,
    session_id: continuum_core::ids::SessionId,
    base_root: PathBuf,
    memory: Option<Arc<dyn continuum_core::memory::MemoryStore>>,
    recovery: Option<Arc<dyn continuum_core::recovery::RecoveryStore>>,
    cancel: CancellationToken,
    reporter: Option<Arc<dyn ExecutionEventSink>>,
    agent_limit: Arc<Semaphore>,
    global_limit: Arc<Semaphore>,
    completed_nodes: Arc<AtomicUsize>,
    total_nodes: usize,
) -> Result<AgentOutcome, PlanError> {
    let _global_permit = global_limit.acquire().await;
    let _agent_permit = agent_limit.acquire().await;
    let task_id = node.id.to_string();
    let agent_name = format!("{:?}", node.agent_kind);
    let workspace = prepare_workspace(&base_root, session_id, &node, reporter.as_ref())?;
    let workspace_path = workspace.path.clone();
    let ctx = ctx
        .with_task_id(node.id)
        .with_workspace(workspace_path.clone())
        .with_reporter(reporter.clone().unwrap_or_else(|| Arc::new(NoopEventSink)));

    if let Some(reporter) = reporter.as_ref() {
        reporter.workspace_prepared(
            node.id,
            node.agent_kind,
            workspace_path.clone(),
            workspace.isolated,
            workspace.source.clone(),
        );
        reporter.task_started(
            node.id,
            node.agent_kind,
            node.label.clone(),
            Some(workspace_path.clone()),
        );
        reporter.task_progress(
            node.id,
            node.agent_kind,
            "dispatching to agent".into(),
            Some(5),
        );
    }

    tracing::info!(agent = ?node.agent_kind, task = %node.id, "dispatching");
    let task = AgentTask::new(node.id, node.payload.clone());
    let outcome = match agent.handle(task, &ctx, cancel.child_token()).await {
        Ok(outcome) => outcome,
        Err(e) => {
            if let Some(reporter) = reporter.as_ref() {
                reporter.task_failed(node.id, node.agent_kind, e.to_string());
            }
            return Err(PlanError::Other(format!("agent error: {e}")));
        }
    };

    if let Some(reporter) = reporter.as_ref() {
        reporter.task_progress(
            node.id,
            node.agent_kind,
            "persisting outcome".into(),
            Some(80),
        );
    }

    write_task_artifacts(&workspace_path, &node, &outcome)?;

    if let Some(ref memory) = memory {
        let item = MemoryItem::new(
            continuum_core::ids::MemoryId::new(),
            serde_json::to_string(&outcome).unwrap_or_default(),
        )
        .with_tag("decision")
        .with_tag(format!("{:?}", node.agent_kind));
        let _ = memory.put(MemoryLayer::Hot, item).await;
    }

    let completed = completed_nodes.fetch_add(1, Ordering::AcqRel) + 1;
    let percent = (completed.checked_mul(100))
        .and_then(|v| v.checked_div(total_nodes))
        .map(|v| v.min(100))
        .unwrap_or(100) as u8;

    if let Some(ref recovery) = recovery {
        let state = SessionState::new(serde_json::json!({
            "completed_nodes": completed,
            "total_nodes": total_nodes,
            "last_agent": agent_name,
            "last_task": task_id,
        }));
        let _ = recovery.checkpoint(session_id, &state).await;
    }

    let conflict = merge_workspace(
        &base_root,
        &workspace_path,
        node.id,
        node.agent_kind,
        reporter.as_ref(),
    )?;

    if workspace.isolated {
        let _ = Command::new("git")
            .args([
                "-C",
                &workspace_path.display().to_string(),
                "worktree",
                "remove",
                "--force",
                &workspace_path.display().to_string(),
            ])
            .output();
    }

    if let Some(reporter) = reporter.as_ref() {
        reporter.task_progress(node.id, node.agent_kind, "completed".into(), Some(percent));
        if let Some(conflict_reason) = conflict {
            reporter.task_blocked(node.id, node.agent_kind, conflict_reason);
        }
        reporter.task_completed(node.id, node.agent_kind, "done".into(), Some(percent));
    }

    Ok(outcome)
}

/// Simple Kahn's algorithm topological sort. Returns node indices in
/// execution order. Errors if the DAG contains a cycle.
fn topological_sort(plan: &ExecutionPlan) -> Result<Vec<usize>, PlanError> {
    let n = plan.nodes.len();
    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    let idx_of: HashMap<_, _> = plan
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id, i))
        .collect();

    for (from, to, _dep) in &plan.edges {
        let fi = idx_of
            .get(from)
            .ok_or_else(|| PlanError::Other(format!("edge source {from} not in plan nodes")))?;
        let ti = idx_of
            .get(to)
            .ok_or_else(|| PlanError::Other(format!("edge target {to} not in plan nodes")))?;
        if *fi != *ti {
            adj[*fi].push(*ti);
            in_degree[*ti] += 1;
        }
    }

    let mut queue: VecDeque<usize> = in_degree
        .iter()
        .enumerate()
        .filter(|(_, &d)| d == 0)
        .map(|(i, _)| i)
        .collect();

    let mut order = Vec::with_capacity(n);
    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &v in &adj[u] {
            in_degree[v] = in_degree[v].saturating_sub(1);
            if in_degree[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    if order.len() != n {
        return Err(PlanError::Other(
            "cycle detected in execution plan DAG".into(),
        ));
    }
    Ok(order)
}
