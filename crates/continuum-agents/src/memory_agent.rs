use std::sync::Arc;

use async_trait::async_trait;

use continuum_core::agent::{
    Agent, AgentCapabilities, AgentContext, AgentError, AgentKind, AgentOutcome, AgentTask,
};
use continuum_core::ids::{AgentId, ModelId};
use continuum_core::memory::{CompressionScope, MemoryStore};
use continuum_core::model::ModelProvider;
use continuum_core::CancellationToken;

/// Agent that compresses hot-tier memory into warm summaries.
/// Triggered by the scheduler when the hot tier exceeds 80% capacity.
pub struct MemoryAgent {
    id: AgentId,
    memory: Arc<dyn MemoryStore>,
    model: Option<Arc<dyn ModelProvider>>,
    model_id: ModelId,
    hot_cap: u64,
}

impl std::fmt::Debug for MemoryAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryAgent")
            .field("id", &self.id)
            .field("model_id", &self.model_id)
            .field("hot_cap", &self.hot_cap)
            .finish()
    }
}

impl MemoryAgent {
    pub fn new(
        memory: Arc<dyn MemoryStore>,
        model: Option<Arc<dyn ModelProvider>>,
        model_id: ModelId,
        hot_cap: u64,
    ) -> Self {
        Self {
            id: AgentId::new(),
            memory,
            model,
            model_id,
            hot_cap,
        }
    }
}

#[async_trait]
impl Agent for MemoryAgent {
    fn id(&self) -> AgentId {
        self.id
    }

    fn kind(&self) -> AgentKind {
        AgentKind::Memory
    }

    fn capabilities(&self) -> AgentCapabilities {
        AgentCapabilities {
            parallel_safe: true,
            needs_sandbox: false,
            needs_network: self.model.is_some(),
            max_concurrency: 1,
        }
    }

    async fn handle(
        &self,
        _task: AgentTask,
        ctx: &AgentContext,
        _cancel: CancellationToken,
    ) -> Result<AgentOutcome, AgentError> {
        // Compress all hot items into warm summaries
        ctx.task_progress(AgentKind::Memory, "starting memory compression", Some(10));
        let report = self
            .memory
            .compress(CompressionScope::AllHot)
            .await
            .map_err(|e| AgentError::Other(format!("compression failed: {e}")))?;
        ctx.task_progress(AgentKind::Memory, "memory items compressed", Some(80));

        tracing::info!(
            processed = report.processed,
            promoted = report.promoted,
            tokens_freed = report.tokens_freed,
            "memory compression complete"
        );

        ctx.task_progress(AgentKind::Memory, "finalizing memory report", Some(95));

        Ok(AgentOutcome::new(serde_json::json!({
            "status": "compressed",
            "agent": "MemoryAgent",
            "processed": report.processed,
            "promoted": report.promoted,
            "tokens_freed": report.tokens_freed,
        })))
    }
}
