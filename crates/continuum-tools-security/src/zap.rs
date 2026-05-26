use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};
use continuum_core::validator::{Finding, Severity};

/// ToolRunner for OWASP ZAP (DAST scanner).
/// Only runs against loopback addresses by default for safety.
#[derive(Debug)]
pub struct Zap;

#[async_trait]
impl ToolRunner for Zap {
    fn id(&self) -> ToolId {
        ToolId::new("zap")
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

        // Determine target URL from invocation args; default to localhost
        let target = invocation
            .args
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("http://localhost:8080");

        // Safety check: refuse non-loopback targets for passive scans
        if invocation.args.get("active_scan").and_then(|v| v.as_bool()).unwrap_or(false) {
            let is_local = target.starts_with("http://localhost")
                || target.starts_with("http://127.0.0.1")
                || target.starts_with("http://[::1]");
            if !is_local {
                return Err(ToolError::Other(
                    "ZAP active scan refused: target is not loopback. Use --allow-target to override.".into(),
                ));
            }
        }

        let argv = vec![
            "zap-cli".into(),
            "--api-key".into(),
            "changeme".into(),
            "quick-scan".into(),
            "--self-contained".into(),
            "--start-options".into(),
            "-config spider.maxDuration=1".into(),
            target.into(),
        ];
        let mut exec = ExecRequest::new(argv);
        exec.cwd = Some(invocation.workspace.clone());

        let stream = sandbox
            .exec(&Cap::grant(), exec)
            .await
            .map_err(|e| ToolError::Sandbox(e.to_string()))?;

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

        let duration = start.elapsed().as_millis() as u64;
        let output = String::from_utf8_lossy(&stdout);
        let findings = parse_zap_output(&output);

        Ok(ToolReport::new(self.id(), findings, exit_code, duration))
    }
}

fn parse_zap_output(output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for line in output.lines() {
        let lower = line.to_lowercase();
        if lower.contains("alert") || lower.contains("risk") || lower.contains("vulnerability") {
            let severity = if lower.contains("high") || lower.contains("critical") {
                Severity::Critical
            } else if lower.contains("medium") {
                Severity::Error
            } else if lower.contains("low") {
                Severity::Warning
            } else {
                Severity::Info
            };
            findings.push(Finding::new("zap", severity, line.to_string(), None, None));
        }
    }
    findings
}
