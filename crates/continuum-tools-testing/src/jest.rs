use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};
use continuum_core::validator::Finding;

/// ToolRunner for Jest (JS/TS unit tests).
#[derive(Debug)]
pub struct JestRunner;

#[async_trait]
impl ToolRunner for JestRunner {
    fn id(&self) -> ToolId {
        ToolId::new("jest")
    }

    fn family(&self) -> ToolFamily {
        ToolFamily::Test
    }

    fn supports(&self, project: &continuum_core::tool::ProjectKind) -> bool {
        project.language == "typescript" || project.language == "javascript"
    }

    async fn run(
        &self,
        invocation: ToolInvocation,
        sandbox: &dyn SandboxHandle,
    ) -> Result<ToolReport, ToolError> {
        let start = std::time::Instant::now();
        invocation.tool_started("starting jest", Some(5));

        let mut argv = vec!["npx".into(), "jest".into(), "--ci".into()];
        if invocation
            .args
            .get("coverage")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            argv.push("--coverage".into());
        }
        if let Some(bail) = invocation.args.get("bail").and_then(|v| v.as_u64()) {
            argv.push("--bail".into());
            argv.push(bail.to_string());
        }

        let mut exec = ExecRequest::new(argv);
        exec.cwd = Some(invocation.workspace.clone());

        let stream = sandbox
            .exec(&Cap::grant(), exec)
            .await
            .map_err(|e| ToolError::Sandbox(e.to_string()))?;
        invocation.tool_progress("jest process launched", Some(25));

        let mut exit_code = 0;
        let mut stream = std::pin::pin!(stream);
        while let Some(event) = stream.next().await {
            if let ExecEvent::Exit(code) = event.map_err(|e| ToolError::Sandbox(e.to_string()))? {
                exit_code = code;
                break;
            }
        }
        invocation.tool_progress("jest process finished", Some(90));

        let duration = start.elapsed().as_millis() as u64;
        let findings = if exit_code == 0 {
            vec![]
        } else {
            vec![Finding::new(
                "jest",
                continuum_core::validator::Severity::Error,
                "jest tests failed",
                None,
                None,
            )]
        };
        invocation.tool_completed(format!("jest finished with exit code {exit_code}"));
        Ok(ToolReport::new(self.id(), findings, exit_code, duration))
    }
}
