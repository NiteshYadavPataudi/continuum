use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};
use continuum_core::validator::Finding;

/// ToolRunner for `cargo test`.
#[derive(Debug)]
pub struct CargoTest;

#[async_trait]
impl ToolRunner for CargoTest {
    fn id(&self) -> ToolId {
        ToolId::new("cargo-test")
    }

    fn family(&self) -> ToolFamily {
        ToolFamily::Test
    }

    fn supports(&self, project: &continuum_core::tool::ProjectKind) -> bool {
        project.language == "rust"
    }

    async fn run(
        &self,
        invocation: ToolInvocation,
        sandbox: &dyn SandboxHandle,
    ) -> Result<ToolReport, ToolError> {
        let start = std::time::Instant::now();

        let mut argv = vec!["cargo".into(), "test".into()];
        if !invocation.paths.is_empty() {
            for p in &invocation.paths {
                argv.push("-p".into());
                argv.push(p.display().to_string());
            }
        }
        argv.push("--".into());
        argv.push("--format=json".into());

        let mut exec = ExecRequest::new(argv);
        exec.cwd = Some(invocation.workspace.clone());

        let stream = sandbox
            .exec(&Cap::grant(), exec)
            .await
            .map_err(|e| ToolError::Sandbox(e.to_string()))?;

        let mut exit_code = 0;
        let mut stream = std::pin::pin!(stream);
        while let Some(event) = stream.next().await {
            if let ExecEvent::Exit(code) = event.map_err(|e| ToolError::Sandbox(e.to_string()))? {
                exit_code = code;
                break;
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        let passed = exit_code == 0;

        let findings = if passed {
            vec![]
        } else {
            vec![Finding::new(
                "cargo-test",
                continuum_core::validator::Severity::Error,
                "tests failed",
                None,
                None,
            )]
        };
        Ok(ToolReport::new(self.id(), findings, exit_code, duration))
    }
}
