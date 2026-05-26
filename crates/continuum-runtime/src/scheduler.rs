use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::sync::Semaphore;

use continuum_core::agent::{Agent, AgentContext, AgentKind, AgentOutcome, AgentTask};
use continuum_core::memory::{MemoryItem, MemoryLayer};
use continuum_core::model::ModelProvider;
use continuum_core::planner::{ExecutionPlan, PlanError};
use continuum_core::recovery::SessionState;
use continuum_core::CancellationToken;

use crate::session::Session;

/// Walks an ExecutionPlan DAG, dispatching each node to the appropriate Agent.
pub struct Scheduler {
    agents: HashMap<AgentKind, Arc<dyn Agent>>,
    concurrency: Arc<Semaphore>,
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
            agents.insert(kind, Arc::new(continuum_agents::StubAgent::new(kind)));
        }
        Self {
            agents,
            concurrency: Arc::new(Semaphore::new(4)),
        }
    }

    /// Create a scheduler injecting concrete agent instances.
    pub fn with_agents(agent_map: HashMap<AgentKind, Arc<dyn Agent>>) -> Self {
        Self {
            agents: agent_map,
            concurrency: Arc::new(Semaphore::new(4)),
        }
    }

    /// Create a scheduler with real agents wired to model providers.
    /// Uses `ModelId::new("default")` as the model identifier.
    /// If `memory` is `None`, a stub `MemoryAgent` is used.
    pub fn with_models(
        model: Option<Arc<dyn ModelProvider>>,
    ) -> Self {
        use continuum_agents::*;
        let model_id = continuum_core::ids::ModelId::new("default");
        let mut agents: HashMap<AgentKind, Arc<dyn Agent>> = HashMap::new();
        agents.insert(AgentKind::Planner, Arc::new(PlannerAgent::new(model.clone(), model_id.clone())));
        agents.insert(AgentKind::Architecture, Arc::new(ArchitectureAgent::new(model.clone(), model_id.clone())));
        agents.insert(AgentKind::Coding, Arc::new(CodingAgent::new(model.clone(), model_id.clone())));
        agents.insert(AgentKind::Testing, Arc::new(TestingAgent::new(model.clone(), model_id.clone())));
        agents.insert(AgentKind::Security, Arc::new(SecurityAgent::new(model.clone(), model_id.clone())));
        agents.insert(AgentKind::Review, Arc::new(ReviewAgent::new(model.clone(), model_id.clone())));
        agents.insert(AgentKind::Recovery, Arc::new(RecoveryAgent::new(model, model_id)));
        Self {
            agents,
            concurrency: Arc::new(Semaphore::new(4)),
        }
    }

    /// Register a custom agent, overriding the default stub for its kind.
    pub fn register(&mut self, agent: Arc<dyn Agent>) {
        self.agents.insert(agent.kind(), agent);
    }

    /// Execute the plan within a session context.
    pub async fn run_with_session(
        &self,
        plan: &ExecutionPlan,
        session: &Session,
        cancel: CancellationToken,
    ) -> Result<Vec<AgentOutcome>, PlanError> {
        let order = topological_sort(plan)?;
        let mut outcomes = Vec::with_capacity(plan.nodes.len());
        let ctx = AgentContext::new();

        for &node_idx in &order {
            if cancel.is_cancelled() {
                return Err(PlanError::Other("execution cancelled".into()));
            }

            let node = &plan.nodes[node_idx];
            let agent = self
                .agents
                .get(&node.agent_kind)
                .ok_or_else(|| PlanError::Other(format!("no agent for {:?}", node.agent_kind)))?;

            let _permit = self.concurrency.acquire().await;
            let task = AgentTask::new(node.id, node.payload.clone());

            tracing::info!(agent = ?node.agent_kind, task = %node.id, "dispatching");
            let outcome = agent
                .handle(task, &ctx, cancel.child_token())
                .await
                .map_err(|e| PlanError::Other(format!("agent error: {e}")))?;

            // Journal to memory if available
            if let Some(ref memory) = session.memory {
                let item = MemoryItem::new(
                    continuum_core::ids::MemoryId::new(),
                    serde_json::to_string(&outcome).unwrap_or_default(),
                )
                .with_tag("decision")
                .with_tag(format!("{:?}", node.agent_kind));
                let _ = memory.put(MemoryLayer::Hot, item).await;
            }

            // Checkpoint after each node if recovery is available
            if let Some(ref recovery) = session.recovery {
                let state = SessionState::new(serde_json::json!({
                    "completed_nodes": outcomes.len(),
                    "total_nodes": plan.nodes.len(),
                    "last_agent": format!("{:?}", node.agent_kind),
                }));
                let _ = recovery.checkpoint(session.id, &state).await;
            }

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
        let fi = idx_of.get(from).ok_or_else(|| {
            PlanError::Other(format!("edge source {from} not in plan nodes"))
        })?;
        let ti = idx_of.get(to).ok_or_else(|| {
            PlanError::Other(format!("edge target {to} not in plan nodes"))
        })?;
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
