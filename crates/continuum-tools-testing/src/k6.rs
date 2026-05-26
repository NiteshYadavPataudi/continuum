use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};
use continuum_core::validator::Finding;

#[derive(Debug)]
pub struct K6LoadRunner;

#[async_trait]
impl ToolRunner for K6LoadRunner {
    fn id(&self) -> ToolId {
        ToolId::new("k6")
    }

    fn family(&self) -> ToolFamily {
        ToolFamily::Test
    }

    fn supports(&self, _project: &continuum_core::tool::ProjectKind) -> bool {
        true
    }

    async fn run(
        &self,
        invocation: ToolInvocation,
        sandbox: &dyn SandboxHandle,
    ) -> Result<ToolReport, ToolError> {
        let start = std::time::Instant::now();

        let script = invocation.args.get("script").and_then(|v| v.as_str()).unwrap_or("k6-script.js");
        let mut argv = vec!["k6".into(), "run".into(), script.into()];
        if let Some(vus) = invocation.args.get("vus").and_then(|v| v.as_u64()) {
            argv.push("--vus".into());
            argv.push(vus.to_string());
        }
        if let Some(duration) = invocation.args.get("duration").and_then(|v| v.as_str()) {
            argv.push("--duration".into());
            argv.push(duration.into());
        }

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
        let findings = if exit_code == 0 {
            vec![]
        } else {
            vec![Finding::new("k6", continuum_core::validator::Severity::Error, "load test failed", None, None)]
        };
        Ok(ToolReport::new(self.id(), findings, exit_code, duration))
    }
}
