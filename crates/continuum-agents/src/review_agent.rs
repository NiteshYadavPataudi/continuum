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

/// Code review agent.
///
/// Reads diffs and code context, then emits structured review comments
/// covering correctness, style, performance, and security. Non-blocking
/// (parallel_safe = true) so it can run alongside other agents.
pub struct ReviewAgent {
    id: AgentId,
    model: Option<Arc<dyn ModelProvider>>,
    model_id: ModelId,
}

impl std::fmt::Debug for ReviewAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReviewAgent")
            .field("id", &self.id)
            .field("model_id", &self.model_id)
            .finish()
    }
}

impl ReviewAgent {
    pub fn new(model: Option<Arc<dyn ModelProvider>>, model_id: ModelId) -> Self {
        Self {
            id: AgentId::new(),
            model,
            model_id,
        }
    }
}

#[async_trait]
impl Agent for ReviewAgent {
    fn id(&self) -> AgentId {
        self.id
    }
    fn kind(&self) -> AgentKind {
        AgentKind::Review
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
        task: AgentTask,
        ctx: &AgentContext,
        cancel: CancellationToken,
    ) -> Result<AgentOutcome, AgentError> {
        let payload = &task.payload;
        let diff = payload
            .get("diff")
            .and_then(|v| v.as_str())
            .unwrap_or("(no diff provided)");
        let context = payload
            .get("context")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let goal = payload
            .get("goal")
            .and_then(|v| v.as_str())
            .unwrap_or("(no goal)");
        ctx.task_progress(AgentKind::Review, "starting review", Some(10));

        let (comments, verdict) = if let Some(ref provider) = self.model {
            ctx.task_progress(AgentKind::Review, "calling model", Some(25));
            let prompt = format!(
                r#"You are a code review agent. Perform a thorough review of the following diff.

Goal the change is meant to achieve: {goal}

Additional context:
{context}

Diff:
```diff
{diff}
```

Review for:
1. Correctness — does it achieve the goal without bugs?
2. Style — idiomatic code, naming, structure
3. Performance — unnecessary allocations, blocking calls, N+1 patterns
4. Security — injection risks, unchecked inputs, secret exposure

Return ONLY valid JSON (no markdown fences):
{{
  "comments": [
    {{
      "severity": "<nit|warning|error>",
      "category": "<correctness|style|performance|security>",
      "line": <line number or null>,
      "message": "<clear actionable description>"
    }}
  ],
  "verdict": "<pass|changes_requested>",
  "summary": "<one paragraph overall assessment>"
}}"#
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
            ctx.task_progress(AgentKind::Review, "model response received", Some(75));

            let json = parse_json_response(&raw);
            let comments = json
                .get("comments")
                .cloned()
                .unwrap_or(serde_json::json!([]));
            let verdict = json
                .get("verdict")
                .and_then(|v| v.as_str())
                .unwrap_or("pass")
                .to_string();
            (comments, verdict)
        } else {
            (serde_json::json!([]), "pass".to_string())
        };

        ctx.task_progress(AgentKind::Review, "finalizing review", Some(95));

        let error_count = comments
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|c| c.get("severity").and_then(|s| s.as_str()) == Some("error"))
                    .count()
            })
            .unwrap_or(0);

        Ok(AgentOutcome::new(serde_json::json!({
            "status": "reviewed",
            "agent": "ReviewAgent",
            "task_id": task.task_id,
            "comments": comments,
            "error_count": error_count,
            "verdict": verdict,
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
