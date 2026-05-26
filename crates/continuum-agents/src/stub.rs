use async_trait::async_trait;
use continuum_core::agent::{Agent, AgentCapabilities, AgentContext, AgentError, AgentKind, AgentOutcome, AgentTask};
use continuum_core::ids::AgentId;
use continuum_core::CancellationToken;

/// A no-op agent that logs its task and returns an empty outcome.
/// Used by the Phase 3 scheduler until real agents land in Phase 4.
#[derive(Debug)]
pub struct StubAgent {
    id: AgentId,
    kind: AgentKind,
}

impl StubAgent {
    /// Create a new stub agent for the given kind.
    pub fn new(kind: AgentKind) -> Self {
        Self {
            id: AgentId::new(),
            kind,
        }
    }
}

#[async_trait]
impl Agent for StubAgent {
    fn id(&self) -> AgentId {
        self.id
    }

    fn kind(&self) -> AgentKind {
        self.kind
    }

    fn capabilities(&self) -> AgentCapabilities {
        AgentCapabilities {
            parallel_safe: true,
            needs_sandbox: false,
            needs_network: false,
            max_concurrency: 1,
        }
    }

    async fn handle(
        &self,
        task: AgentTask,
        _ctx: &AgentContext,
        _cancel: CancellationToken,
    ) -> Result<AgentOutcome, AgentError> {
        tracing::info!(
            agent = ?self.kind,
            task = %task.task_id,
            "stub agent handling task"
        );
        Ok(AgentOutcome::new(serde_json::json!({
            "status": "stub",
            "agent": format!("{:?}", self.kind),
            "task_id": task.task_id,
        })))
    }
}
