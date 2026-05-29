use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;

use continuum_core::caps::Cap;
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};
use continuum_core::validator::{Finding, Severity};

/// ToolRunner for `cargo clippy`.
#[derive(Debug)]
pub struct Clippy;

#[async_trait]
impl ToolRunner for Clippy {
    fn id(&self) -> ToolId {
        ToolId::new("clippy")
    }

    fn family(&self) -> ToolFamily {
        ToolFamily::Linter
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
        invocation.tool_started("starting clippy", Some(5));

        let mut argv = vec![
            "cargo".into(),
            "clippy".into(),
            "--message-format=json".into(),
        ];
        if !invocation.paths.is_empty() {
            argv.push("-p".into());
            for p in &invocation.paths {
                argv.push(p.display().to_string());
            }
        }

        let mut exec = ExecRequest::new(argv);
        exec.cwd = Some(invocation.workspace.clone());

        let stream = sandbox
            .exec(&Cap::grant(), exec)
            .await
            .map_err(|e| ToolError::Sandbox(e.to_string()))?;
        invocation.tool_progress("clippy process launched", Some(20));

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = 0;

        let mut stream = std::pin::pin!(stream);
        while let Some(event) = stream.next().await {
            match event.map_err(|e| ToolError::Sandbox(e.to_string()))? {
                ExecEvent::Stdout(bytes) => stdout.extend_from_slice(&bytes),
                ExecEvent::Stderr(bytes) => stderr.extend_from_slice(&bytes),
                ExecEvent::Exit(code) => {
                    exit_code = code;
                    break;
                }
                _ => {}
            }
        }
        invocation.tool_progress("clippy process finished", Some(90));

        let duration = start.elapsed().as_millis() as u64;
        let output = String::from_utf8_lossy(&stdout);
        let findings = parse_clippy_json(&output);

        invocation.tool_completed(format!("clippy finished with exit code {exit_code}"));
        Ok(ToolReport::new(self.id(), findings, exit_code, duration))
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ClippyMessage {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    message: Option<ClippyDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct ClippyDiagnostic {
    message: Option<String>,
    code: Option<ClippyCode>,
    level: Option<String>,
    spans: Option<Vec<ClippySpan>>,
}

#[derive(Debug, Deserialize)]
struct ClippyCode {
    code: String,
}

#[derive(Debug, Deserialize)]
struct ClippySpan {
    file_name: Option<String>,
    line_start: Option<u32>,
}

fn parse_clippy_json(output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let msg: ClippyMessage = match serde_json::from_str(line) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let diag = match msg.message {
            Some(d) => d,
            None => continue,
        };

        let is_clippy = diag
            .code
            .as_ref()
            .map(|c| c.code.starts_with("clippy::"))
            .unwrap_or(false);

        if !is_clippy {
            continue;
        }

        let severity = match diag.level.as_deref() {
            Some("error") => Severity::Error,
            Some("warning") => Severity::Warning,
            Some("help") | Some("note") => Severity::Info,
            _ => Severity::Warning,
        };

        let span = diag.spans.and_then(|s| s.into_iter().next());
        let file = span
            .as_ref()
            .and_then(|s| s.file_name.clone())
            .map(std::path::PathBuf::from);
        let line = span.and_then(|s| s.line_start);

        findings.push(Finding::new(
            "clippy",
            severity,
            diag.message.unwrap_or_default(),
            file,
            line,
        ));
    }

    findings
}
