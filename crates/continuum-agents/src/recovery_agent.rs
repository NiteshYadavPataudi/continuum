use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::agent::{
    Agent, AgentCapabilities, AgentContext, AgentError, AgentKind, AgentOutcome, AgentTask,
};
use continuum_core::caps::Cap;
use continuum_core::ids::{AgentId, ModelId};
use continuum_core::model::{CompletionRequest, Message, ModelProvider};
use continuum_core::CancellationToken;

/// Recovery agent.
///
/// Invoked when the heartbeat detects a stuck task. Reads replay events and
/// the error message, then proposes one of three recovery actions:
/// - `retry`: re-run the same task with a hint
/// - `skip`: mark the task done and proceed
/// - `replan`: hand off to PlannerAgent for a full re-plan
pub struct RecoveryAgent {
    id: AgentId,
    model: Option<Arc<dyn ModelProvider>>,
    model_id: ModelId,
}

impl std::fmt::Debug for RecoveryAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecoveryAgent")
            .field("id", &self.id)
            .field("model_id", &self.model_id)
            .finish()
    }
}

impl RecoveryAgent {
    pub fn new(model: Option<Arc<dyn ModelProvider>>, model_id: ModelId) -> Self {
        Self {
            id: AgentId::new(),
            model,
            model_id,
        }
    }
}

#[async_trait]
impl Agent for RecoveryAgent {
    fn id(&self) -> AgentId {
        self.id
    }
    fn kind(&self) -> AgentKind {
        AgentKind::Recovery
    }

    fn capabilities(&self) -> AgentCapabilities {
        AgentCapabilities {
            parallel_safe: false,
            needs_sandbox: false,
            needs_network: self.model.is_some(),
            max_concurrency: 1,
        }
    }

    async fn handle(
        &self,
        task: AgentTask,
        _ctx: &AgentContext,
        cancel: CancellationToken,
    ) -> Result<AgentOutcome, AgentError> {
        let payload = &task.payload;
        let stuck_task = payload
            .get("stuck_task")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let stuck_label = stuck_task
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown task");
        let error_message = payload
            .get("error_message")
            .and_then(|v| v.as_str())
            .unwrap_or("(no error message)");
        let replay_events = payload
            .get("replay_events")
            .cloned()
            .unwrap_or(serde_json::json!([]));

        let (action, reasoning, retry_hint) = if let Some(ref provider) = self.model {
            let prompt = format!(
                r#"You are a recovery agent. A task is stuck and you must decide how to recover.

Stuck task: {stuck_label}
Error message: {error_message}
Replay events (recent actions): {events}

Decide on a recovery strategy:
- "retry": the failure looks transient or could succeed with a different approach
- "skip": the task is not critical and skipping it is safe
- "replan": the task reveals a fundamental flaw that requires replanning

Return ONLY valid JSON (no markdown fences):
{{
  "action": "<retry|skip|replan>",
  "reasoning": "<one paragraph explaining the decision>",
  "retry_hint": "<specific suggestion for the next attempt, or empty string if not retrying>"
}}"#,
                events = replay_events,
            );

            let req = CompletionRequest::new(
                self.model_id.clone(),
                vec![Message::new("user", &prompt)],
            );
            let mut stream = provider
                .complete(&Cap::grant(), req, cancel.child_token())
                .await
                .map_err(|e| AgentError::Other(format!("model call failed: {e}")))?;

            let mut raw = String::new();
            while let Some(delta) = stream.next().await {
                match delta {
                    Ok(d) => raw.push_str(&d.text),
                    Err(e) => return Err(AgentError::Other(format!("stream error: {e}"))),
                }
            }

            let json = parse_json_response(&raw);
            let action = json
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("retry")
                .to_string();
            let reasoning = json
                .get("reasoning")
                .and_then(|v| v.as_str())
                .unwrap_or("Defaulting to retry.")
                .to_string();
            let hint = json
                .get("retry_hint")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (action, reasoning, hint)
        } else {
            (
                "retry".to_string(),
                format!("No model configured — defaulting to retry for: {stuck_label}"),
                String::new(),
            )
        };

        Ok(AgentOutcome::new(serde_json::json!({
            "status": "recovered",
            "agent": "RecoveryAgent",
            "task_id": task.task_id,
            "stuck_task": stuck_task,
            "action": action,
            "reasoning": reasoning,
            "retry_hint": retry_hint,
        })))
    }
}

fn parse_json_response(raw: &str) -> serde_json::Value {
    let s = raw.trim();
    let s = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```"))
        .map(|inner| inner.trim_end_matches("```").trim())
        .unwrap_or(s);
    serde_json::from_str(s).unwrap_or(serde_json::json!({}))
}
