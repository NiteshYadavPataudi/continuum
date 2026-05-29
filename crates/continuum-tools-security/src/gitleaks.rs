use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};
use continuum_core::validator::{Finding, Severity};

/// ToolRunner for Gitleaks (secret scanner).
#[derive(Debug)]
pub struct Gitleaks;

#[async_trait]
impl ToolRunner for Gitleaks {
    fn id(&self) -> ToolId {
        ToolId::new("gitleaks")
    }
    fn family(&self) -> ToolFamily {
        ToolFamily::Security
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
        invocation.tool_started("starting gitleaks", Some(5));

        let argv = vec![
            "gitleaks".into(),
            "detect".into(),
            "--no-color".into(),
            "--no-git".into(),
            "--verbose".into(),
            ".".into(),
        ];
        let mut exec = ExecRequest::new(argv);
        exec.cwd = Some(invocation.workspace.clone());

        let stream = sandbox
            .exec(&Cap::grant(), exec)
            .await
            .map_err(|e| ToolError::Sandbox(e.to_string()))?;
        invocation.tool_progress("gitleaks launched", Some(20));

        let mut stdout = Vec::new();
        let mut exit_code = 0;
        let mut stream = std::pin::pin!(stream);
        while let Some(event) = stream.next().await {
            match event.map_err(|e| ToolError::Sandbox(e.to_string()))? {
                ExecEvent::Stdout(bytes) => stdout.extend_from_slice(&bytes),
                ExecEvent::Exit(code) => {
                    exit_code = code;
                    break;
                }
                _ => {}
            }
        }
        invocation.tool_progress("gitleaks finished", Some(90));

        let duration = start.elapsed().as_millis() as u64;
        let output = String::from_utf8_lossy(&stdout);
        let findings = parse_gitleaks_output(&output);

        invocation.tool_completed(format!("gitleaks finished with exit code {exit_code}"));
        Ok(ToolReport::new(self.id(), findings, exit_code, duration))
    }
}

fn parse_gitleaks_output(output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for line in output.lines() {
        if line.to_lowercase().contains("leak") || line.to_lowercase().contains("secret") {
            findings.push(Finding::new(
                "gitleaks",
                Severity::Critical,
                line.to_string(),
                None,
                None,
            ));
        }
    }
    findings
}
