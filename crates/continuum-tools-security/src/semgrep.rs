use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};
use continuum_core::validator::{Finding, Severity};

/// ToolRunner for Semgrep (static analysis).
#[derive(Debug)]
pub struct Semgrep;

#[async_trait]
impl ToolRunner for Semgrep {
    fn id(&self) -> ToolId {
        ToolId::new("semgrep")
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
        invocation.tool_started("starting semgrep", Some(5));
        let argv = vec![
            "semgrep".into(),
            "--config=auto".into(),
            "--json".into(),
            ".".into(),
        ];
        let mut exec = ExecRequest::new(argv);
        exec.cwd = Some(invocation.workspace.clone());

        let stream = sandbox
            .exec(&Cap::grant(), exec)
            .await
            .map_err(|e| ToolError::Sandbox(e.to_string()))?;
        invocation.tool_progress("semgrep process launched", Some(20));

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
        invocation.tool_progress("semgrep process finished", Some(90));

        let duration = start.elapsed().as_millis() as u64;
        let output = String::from_utf8_lossy(&stdout);
        let findings = parse_semgrep_output(&output);

        invocation.tool_completed(format!("semgrep finished with exit code {exit_code}"));
        Ok(ToolReport::new(self.id(), findings, exit_code, duration))
    }
}

fn parse_semgrep_output(output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(results) = json.get("results").and_then(|v| v.as_array()) {
            for r in results {
                let msg = r
                    .get("extra")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("semgrep finding");
                let path = r.get("path").and_then(|p| p.as_str());
                let start = r
                    .get("start")
                    .and_then(|s| s.get("line"))
                    .and_then(|l| l.as_u64());
                findings.push(Finding::new(
                    "semgrep",
                    Severity::Warning,
                    msg,
                    path.map(std::path::PathBuf::from),
                    start.map(|l| l as u32),
                ));
            }
        }
    }
    findings
}
