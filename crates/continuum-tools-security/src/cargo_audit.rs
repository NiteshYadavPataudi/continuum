use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};
use continuum_core::validator::{Finding, Severity};

/// ToolRunner for `cargo audit` (security vulnerability scanner).
#[derive(Debug)]
pub struct CargoAudit;

#[async_trait]
impl ToolRunner for CargoAudit {
    fn id(&self) -> ToolId {
        ToolId::new("cargo-audit")
    }

    fn family(&self) -> ToolFamily {
        ToolFamily::Security
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
        invocation.tool_started("starting cargo audit", Some(5));

        let argv = vec!["cargo".into(), "audit".into()];

        let mut exec = ExecRequest::new(argv);
        exec.cwd = Some(invocation.workspace.clone());

        let stream = sandbox
            .exec(&Cap::grant(), exec)
            .await
            .map_err(|e| ToolError::Sandbox(e.to_string()))?;
        invocation.tool_progress("cargo audit launched", Some(20));

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
        invocation.tool_progress("cargo audit finished", Some(90));

        let duration = start.elapsed().as_millis() as u64;
        let output = String::from_utf8_lossy(&stdout);
        let findings = parse_audit_output(&output);

        invocation.tool_completed(format!("cargo audit finished with exit code {exit_code}"));
        Ok(ToolReport::new(self.id(), findings, exit_code, duration))
    }
}

fn parse_audit_output(output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for line in output.lines() {
        let lower = line.to_lowercase();
        if lower.contains("error") || lower.contains("warning") {
            findings.push(Finding::new(
                "cargo-audit",
                if lower.contains("error") {
                    Severity::Critical
                } else {
                    Severity::Warning
                },
                line.to_string(),
                None,
                None,
            ));
        }
    }
    findings
}
