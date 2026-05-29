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

/// Testing agent: generates idiomatic test cases for produced code.
///
/// Accepts the implementation code and goal, then writes test cases in the
/// same language. Can be wired to a sandbox runner to execute the tests.
pub struct TestingAgent {
    id: AgentId,
    model: Option<Arc<dyn ModelProvider>>,
    model_id: ModelId,
}

impl std::fmt::Debug for TestingAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestingAgent")
            .field("id", &self.id)
            .field("model_id", &self.model_id)
            .finish()
    }
}

impl TestingAgent {
    pub fn new(model: Option<Arc<dyn ModelProvider>>, model_id: ModelId) -> Self {
        Self {
            id: AgentId::new(),
            model,
            model_id,
        }
    }
}

#[async_trait]
impl Agent for TestingAgent {
    fn id(&self) -> AgentId {
        self.id
    }
    fn kind(&self) -> AgentKind {
        AgentKind::Testing
    }

    fn capabilities(&self) -> AgentCapabilities {
        AgentCapabilities {
            parallel_safe: false,
            needs_sandbox: true,
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
        let goal = payload
            .get("goal")
            .and_then(|v| v.as_str())
            .unwrap_or("implement the described change");
        let code = payload
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("(no implementation provided yet)");
        let language = payload
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("rust");
        ctx.task_progress(AgentKind::Testing, "starting test generation", Some(10));

        let tests = if let Some(ref provider) = self.model {
            ctx.task_progress(AgentKind::Testing, "calling model", Some(25));
            let prompt = format!(
                r#"You are a testing agent. Write comprehensive test cases for the following code.

Goal: {goal}
Language: {language}

Implementation:
```{language}
{code}
```

Requirements:
- Write idiomatic {language} tests
- Cover the happy path and at least two edge cases
- For Rust: use #[cfg(test)] module with #[test] functions
- For TypeScript/JavaScript: use describe/it blocks
- Do not include the implementation in your output, only the tests
- Return ONLY a code block wrapped in markdown fences"#
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
            ctx.task_progress(AgentKind::Testing, "model response received", Some(75));

            extract_code_block(&raw).unwrap_or(raw)
        } else {
            format!(
                "// Stub tests for: {goal}\n// No model provider configured.\n#[cfg(test)]\nmod tests {{\n    #[test]\n    fn placeholder() {{ todo!() }}\n}}\n"
            )
        };

        ctx.task_progress(AgentKind::Testing, "finalizing test output", Some(95));

        let test_count = count_test_functions(&tests, language);

        Ok(AgentOutcome::new(serde_json::json!({
            "status": "tests_written",
            "agent": "TestingAgent",
            "task_id": task.task_id,
            "goal": goal,
            "language": language,
            "tests": tests,
            "test_count": test_count,
        })))
    }
}

fn extract_code_block(text: &str) -> Option<String> {
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
        None
    } else {
        Some(blocks.join("\n"))
    }
}

fn count_test_functions(code: &str, language: &str) -> usize {
    match language {
        "rust" => code.matches("#[test]").count(),
        "typescript" | "javascript" => code.matches("it(").count() + code.matches("test(").count(),
        "python" => code.matches("def test_").count(),
        _ => code.matches("#[test]").count() + code.matches("def test_").count(),
    }
}
