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

/// Coding agent that uses a model provider to generate code.
pub struct CodingAgent {
    id: AgentId,
    model: Option<Arc<dyn ModelProvider>>,
    model_id: ModelId,
}

impl std::fmt::Debug for CodingAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodingAgent")
            .field("id", &self.id)
            .field("model_id", &self.model_id)
            .field("model", &self.model.as_ref().map(|_| "<provider>"))
            .finish()
    }
}

impl CodingAgent {
    /// Create a coding agent with an optional model provider.
    pub fn new(model: Option<Arc<dyn ModelProvider>>, model_id: ModelId) -> Self {
        Self {
            id: AgentId::new(),
            model,
            model_id,
        }
    }

    /// Return the model provider reference, if any.
    pub fn model(&self) -> Option<&Arc<dyn ModelProvider>> {
        self.model.as_ref()
    }
}

#[async_trait]
impl Agent for CodingAgent {
    fn id(&self) -> AgentId {
        self.id
    }

    fn kind(&self) -> AgentKind {
        AgentKind::Coding
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
        let goal = task
            .payload
            .get("goal")
            .and_then(|v| v.as_str())
            .unwrap_or("implement the described change");
        ctx.task_progress(AgentKind::Coding, "starting code generation", Some(10));

        let code = if let Some(ref provider) = self.model {
            ctx.task_progress(AgentKind::Coding, "calling model", Some(25));
            let prompt = format!(
                "You are a coding agent. Implement the following goal.\n\
                 Write production-quality, idiomatic Rust code.\n\n\
                 Goal: {goal}\n\n\
                 Return ONLY the code, wrapped in a markdown code block."
            );

            let req =
                CompletionRequest::new(self.model_id.clone(), vec![Message::new("user", &prompt)]);

            let mut stream = provider
                .complete(&Cap::grant(), req, cancel.child_token())
                .await
                .map_err(|e| AgentError::Other(format!("model call failed: {e}")))?;

            let mut response = String::new();
            while let Some(delta) = stream.next().await {
                match delta {
                    Ok(d) => response.push_str(&d.text),
                    Err(e) => return Err(AgentError::Other(format!("stream error: {e}"))),
                }
            }
            ctx.task_progress(AgentKind::Coding, "model response received", Some(75));

            extract_code_blocks(&response).unwrap_or(response)
        } else {
            format!("// Stub implementation for: {goal}\n// No model provider configured.\n")
        };

        ctx.task_progress(AgentKind::Coding, "finalizing code output", Some(95));

        Ok(AgentOutcome::new(serde_json::json!({
            "status": "generated",
            "agent": "CodingAgent",
            "task_id": task.task_id,
            "goal": goal,
            "code": code,
        })))
    }
}

fn extract_code_blocks(text: &str) -> Option<String> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        if line.starts_with("```") {
            in_block = !in_block;
            continue;
        }
        if in_block {
            blocks.push(line);
        }
    }
    if blocks.is_empty() {
        return None;
    }
    Some(blocks.join("\n"))
}
