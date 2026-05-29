use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};
use continuum_core::validator::{Finding, Severity};

/// ToolRunner for Trivy (vulnerability scanner).
#[derive(Debug)]
pub struct Trivy;

#[async_trait]
impl ToolRunner for Trivy {
    fn id(&self) -> ToolId {
        ToolId::new("trivy")
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
        invocation.tool_started("starting trivy", Some(5));

        let argv = vec![
            "trivy".into(),
            "fs".into(),
            "--format=json".into(),
            invocation.workspace.display().to_string(),
        ];
        let mut exec = ExecRequest::new(argv);
        exec.cwd = Some(invocation.workspace.clone());

        let stream = sandbox
            .exec(&Cap::grant(), exec)
            .await
            .map_err(|e| ToolError::Sandbox(e.to_string()))?;
        invocation.tool_progress("trivy launched", Some(20));

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
        invocation.tool_progress("trivy finished", Some(90));

        let duration = start.elapsed().as_millis() as u64;
        let output = String::from_utf8_lossy(&stdout);
        let findings = parse_trivy_output(&output);

        invocation.tool_completed(format!("trivy finished with exit code {exit_code}"));
        Ok(ToolReport::new(self.id(), findings, exit_code, duration))
    }
}

fn parse_trivy_output(output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(results) = json.get("Results").and_then(|v| v.as_array()) {
            for result in results {
                if let Some(vulns) = result.get("Vulnerabilities").and_then(|v| v.as_array()) {
                    for v in vulns {
                        let pkg = v.get("PkgName").and_then(|s| s.as_str()).unwrap_or("?");
                        let severity_str = v
                            .get("Severity")
                            .and_then(|s| s.as_str())
                            .unwrap_or("UNKNOWN");
                        let severity = match severity_str {
                            "CRITICAL" => Severity::Critical,
                            "HIGH" => Severity::Error,
                            "MEDIUM" | "LOW" => Severity::Warning,
                            _ => Severity::Info,
                        };
                        let vuln_id = v
                            .get("VulnerabilityID")
                            .and_then(|s| s.as_str())
                            .unwrap_or("?");
                        let title = v.get("Title").and_then(|s| s.as_str()).unwrap_or("");
                        findings.push(Finding::new(
                            "trivy",
                            severity,
                            format!("{vuln_id} in {pkg}: {title}"),
                            None,
                            None,
                        ));
                    }
                }
            }
        }
    }
    findings
}
