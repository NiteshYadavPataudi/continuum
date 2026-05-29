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

/// Re-planning agent invoked when a stuck signal fires.
///
/// Reads the task history and stuck reason, then proposes a revised set of
/// task nodes to replace the stalled portion of the plan.
pub struct PlannerAgent {
    id: AgentId,
    model: Option<Arc<dyn ModelProvider>>,
    model_id: ModelId,
}

impl std::fmt::Debug for PlannerAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlannerAgent")
            .field("id", &self.id)
            .field("model_id", &self.model_id)
            .finish()
    }
}

impl PlannerAgent {
    pub fn new(model: Option<Arc<dyn ModelProvider>>, model_id: ModelId) -> Self {
        Self {
            id: AgentId::new(),
            model,
            model_id,
        }
    }
}

#[async_trait]
impl Agent for PlannerAgent {
    fn id(&self) -> AgentId {
        self.id
    }
    fn kind(&self) -> AgentKind {
        AgentKind::Planner
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
        ctx: &AgentContext,
        cancel: CancellationToken,
    ) -> Result<AgentOutcome, AgentError> {
        let payload = &task.payload;
        let stuck_reason = payload
            .get("stuck_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown reason");
        let original_goal = payload
            .get("original_goal")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown goal");
        let task_history = payload
            .get("task_history")
            .cloned()
            .unwrap_or(serde_json::json!([]));
        ctx.task_progress(AgentKind::Planner, "replanning started", Some(10));

        let (revised_tasks, reasoning) = if let Some(ref provider) = self.model {
            ctx.task_progress(AgentKind::Planner, "calling model", Some(25));
            let prompt = format!(
                r#"You are a re-planning agent. The current execution plan is stuck.

Original goal: {original_goal}
Stuck reason: {stuck_reason}
Task history (completed/failed tasks): {history}

Produce a revised plan to get the goal back on track. Avoid repeating failed approaches.
Return ONLY valid JSON (no markdown fences):
{{
  "reasoning": "<one paragraph explaining the revised strategy>",
  "revised_tasks": [
    {{
      "agent": "<Coding|Testing|Architecture|Security|Review|Recovery>",
      "label": "<concise task description>",
      "depends_on": [<0-based indices within revised_tasks>]
    }}
  ]
}}"#,
                history = task_history,
            );

            let req =
                CompletionRequest::new(self.model_id.clone(), vec![Message::new("user", &prompt)]);
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
            ctx.task_progress(AgentKind::Planner, "model response received", Some(75));

            let json = parse_json_response(&raw);
            let reasoning = json
                .get("reasoning")
                .and_then(|v| v.as_str())
                .unwrap_or("Revised plan generated.")
                .to_string();
            let tasks = json
                .get("revised_tasks")
                .cloned()
                .unwrap_or(serde_json::json!([]));
            (tasks, reasoning)
        } else {
            (
                serde_json::json!([
                    { "agent": "Coding", "label": format!("Retry: {original_goal}"), "depends_on": [] }
                ]),
                format!(
                    "No model available — defaulting to single Coding retry for: {original_goal}"
                ),
            )
        };

        ctx.task_progress(AgentKind::Planner, "finalizing revised plan", Some(95));

        Ok(AgentOutcome::new(serde_json::json!({
            "status": "replanned",
            "agent": "PlannerAgent",
            "task_id": task.task_id,
            "original_goal": original_goal,
            "stuck_reason": stuck_reason,
            "reasoning": reasoning,
            "revised_tasks": revised_tasks,
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
