use std::path::Path;

use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};

/// Playwright-backed browser automation runner.
///
/// Stages a Node.js bootstrap script in the sandbox and drives
/// Playwright via stdin/stdout JSON protocol.
#[derive(Debug)]
pub struct PlaywrightRunner;

impl PlaywrightRunner {
    const BOOTSTRAP: &'static [u8] = include_bytes!("playwright-bootstrap.js");
}

#[async_trait]
impl ToolRunner for PlaywrightRunner {
    fn id(&self) -> ToolId {
        ToolId::new("playwright")
    }

    fn family(&self) -> ToolFamily {
        ToolFamily::Browser
    }

    fn supports(&self, project: &continuum_core::tool::ProjectKind) -> bool {
        project.language == "javascript" || project.language == "typescript"
    }

    async fn run(
        &self,
        invocation: ToolInvocation,
        sandbox: &dyn SandboxHandle,
    ) -> Result<ToolReport, ToolError> {
        let start = std::time::Instant::now();
        invocation.tool_started("starting playwright", Some(5));

        // Write bootstrap script into sandbox
        let bootstrap_path = "/tmp/playwright-bootstrap.js";
        sandbox
            .write_file(Path::new(bootstrap_path), Self::BOOTSTRAP)
            .await
            .map_err(|e| ToolError::Sandbox(format!("write bootstrap: {e}")))?;

        // Invoke node with the invocation payload as JSON arg
        let payload = serde_json::to_string(&invocation.args)
            .map_err(|e| ToolError::Other(format!("serialize args: {e}")))?;

        let mut exec = ExecRequest::new(vec!["node".into(), bootstrap_path.into(), payload]);
        exec.cwd = Some(invocation.workspace.clone());

        let stream = sandbox
            .exec(&Cap::grant(), exec)
            .await
            .map_err(|e| ToolError::Sandbox(e.to_string()))?;
        invocation.tool_progress("playwright launched", Some(25));

        let mut exit_code = 0;
        let mut stream = std::pin::pin!(stream);
        while let Some(event) = stream.next().await {
            if let ExecEvent::Exit(code) = event.map_err(|e| ToolError::Sandbox(e.to_string()))? {
                exit_code = code;
                break;
            }
        }
        invocation.tool_progress("playwright finished", Some(90));

        let duration = start.elapsed().as_millis() as u64;
        let _passed = exit_code == 0;

        invocation.tool_completed(format!("playwright finished with exit code {exit_code}"));
        Ok(ToolReport::new(self.id(), vec![], exit_code, duration))
    }
}
