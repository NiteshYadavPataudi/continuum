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

/// Architecture review agent.
///
/// Reads the proposed diff against architectural principles (SOLID, crate
/// isolation, no cross-layer imports) and emits structured findings.
pub struct ArchitectureAgent {
    id: AgentId,
    model: Option<Arc<dyn ModelProvider>>,
    model_id: ModelId,
}

impl std::fmt::Debug for ArchitectureAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArchitectureAgent")
            .field("id", &self.id)
            .field("model_id", &self.model_id)
            .finish()
    }
}

impl ArchitectureAgent {
    pub fn new(model: Option<Arc<dyn ModelProvider>>, model_id: ModelId) -> Self {
        Self {
            id: AgentId::new(),
            model,
            model_id,
        }
    }
}

#[async_trait]
impl Agent for ArchitectureAgent {
    fn id(&self) -> AgentId {
        self.id
    }
    fn kind(&self) -> AgentKind {
        AgentKind::Architecture
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
        let goal = payload
            .get("goal")
            .and_then(|v| v.as_str())
            .unwrap_or("(no goal)");
        let arch_summary = payload
            .get("arch_summary")
            .and_then(|v| v.as_str())
            .unwrap_or("(no architecture summary provided)");
        ctx.task_progress(
            AgentKind::Architecture,
            "starting architecture review",
            Some(10),
        );

        let (findings, verdict) = if let Some(ref provider) = self.model {
            ctx.task_progress(AgentKind::Architecture, "calling model", Some(25));
            let prompt = format!(
                r#"You are an architecture review agent. Review the following diff for architectural issues.

Architecture principles to enforce:
- SOLID principles (especially Single Responsibility and Dependency Inversion)
- Crate isolation: lower-layer crates must not import upper-layer crates
- No cross-layer imports (e.g. storage crates must not import agent crates)
- Minimal public API surface — prefer restricted visibility
- Async-safe: no blocking calls on the async executor

Goal: {goal}
Architecture summary: {arch_summary}

Diff to review:
```diff
{diff}
```

Return ONLY valid JSON (no markdown fences):
{{
  "findings": [
    {{
      "severity": "<error|warning|info>",
      "message": "<clear description of the issue>",
      "location": "<file:line or 'general' if not file-specific>"
    }}
  ],
  "verdict": "<pass|fail>",
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
            ctx.task_progress(AgentKind::Architecture, "model response received", Some(75));

            let json = parse_json_response(&raw);
            let findings = json
                .get("findings")
                .cloned()
                .unwrap_or(serde_json::json!([]));
            let verdict = json
                .get("verdict")
                .and_then(|v| v.as_str())
                .unwrap_or("pass")
                .to_string();
            (findings, verdict)
        } else {
            (serde_json::json!([]), "pass".to_string())
        };

        ctx.task_progress(AgentKind::Architecture, "finalizing review", Some(95));

        let error_count = findings
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|f| f.get("severity").and_then(|s| s.as_str()) == Some("error"))
                    .count()
            })
            .unwrap_or(0);

        Ok(AgentOutcome::new(serde_json::json!({
            "status": "reviewed",
            "agent": "ArchitectureAgent",
            "task_id": task.task_id,
            "findings": findings,
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
