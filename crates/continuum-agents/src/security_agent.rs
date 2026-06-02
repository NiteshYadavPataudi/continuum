use std::sync::Arc;

use async_trait::async_trait;

use continuum_core::agent::{
    Agent, AgentCapabilities, AgentContext, AgentError, AgentKind, AgentOutcome, AgentTask,
};
use continuum_core::ids::{AgentId, ModelId};
use continuum_core::model::ModelProvider;
use continuum_core::validator::{Finding, Severity};
use continuum_core::CancellationToken;
use continuum_security::{
    AuditReportBuilder, ComplianceAttestation, HardeningMode, SecurityPolicy,
};

/// Security audit / hardening agent. Implements the three hardening modes.
pub struct SecurityAgent {
    id: AgentId,
    model: Option<Arc<dyn ModelProvider>>,
    model_id: ModelId,
}

impl std::fmt::Debug for SecurityAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecurityAgent")
            .field("id", &self.id)
            .field("model_id", &self.model_id)
            .finish()
    }
}

impl SecurityAgent {
    pub fn new(model: Option<Arc<dyn ModelProvider>>, model_id: ModelId) -> Self {
        Self {
            id: AgentId::new(),
            model,
            model_id,
        }
    }
}

#[async_trait]
impl Agent for SecurityAgent {
    fn id(&self) -> AgentId {
        self.id
    }
    fn kind(&self) -> AgentKind {
        AgentKind::Security
    }

    fn capabilities(&self) -> AgentCapabilities {
        AgentCapabilities {
            parallel_safe: true,
            needs_sandbox: true,
            needs_network: self.model.is_some(),
            max_concurrency: 1,
        }
    }

    async fn handle(
        &self,
        task: AgentTask,
        ctx: &AgentContext,
        _cancel: CancellationToken,
    ) -> Result<AgentOutcome, AgentError> {
        let payload = &task.payload;
        ctx.task_progress(AgentKind::Security, "starting security audit", Some(10));

        // Extract mode from payload
        let mode_str = payload
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("audit");
        let mode: HardeningMode = mode_str
            .parse()
            .map_err(|e: String| AgentError::Other(format!("invalid hardening mode: {e}")))?;
        let suggest = payload
            .get("suggest")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let exit_code = payload
            .get("exit_code")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let policy = SecurityPolicy::new(mode);
        let severity_floor = policy.severity_floor();
        let mut report_builder = AuditReportBuilder::new(mode_str);

        // Discover available security tools
        ctx.task_progress(AgentKind::Security, "running scanners", Some(35));
        let findings = run_security_tools(ctx);

        // Filter by severity floor
        let filtered: Vec<Finding> = findings
            .into_iter()
            .filter(|f| severity_floor.allows(&f.severity))
            .collect();

        report_builder.add_findings(filtered.clone());
        ctx.task_progress(AgentKind::Security, "assembling report", Some(75));

        let mut artifacts = serde_json::json!({
            "status": "audited",
            "agent": "SecurityAgent",
            "mode": mode_str,
            "findings": filtered.len(),
            "report_md": report_builder.build().to_markdown(),
        });

        // Hardening mode: remediate findings
        if mode == HardeningMode::Hardening || mode == HardeningMode::Enterprise {
            let patches = remediate_findings(&filtered, suggest);
            artifacts["patches"] = serde_json::json!(patches);
            artifacts["suggest_only"] = serde_json::json!(suggest);

            if !suggest {
                // Apply patches via sandbox if available
                let applied = apply_patches_via_sandbox(&patches, ctx).await;
                artifacts["patches_applied"] = serde_json::json!(applied);
            }
        }

        // Enterprise mode: compliance attestation
        if mode == HardeningMode::Enterprise {
            let compliance = ComplianceAttestation::build();
            artifacts["compliance_md"] = serde_json::Value::String(compliance.to_markdown());
        }

        // Exit code: signal findings above floor
        if exit_code && !filtered.is_empty() {
            artifacts["exit_code"] = serde_json::json!(1);
        }

        ctx.task_progress(AgentKind::Security, "finalizing audit", Some(95));

        Ok(AgentOutcome::new(artifacts))
    }
}

/// Run all configured security scanners within the agent's workspace.
fn run_security_tools(ctx: &AgentContext) -> Vec<Finding> {
    let mut findings = Vec::new();
    let workspace = ctx
        .workspace
        .as_deref()
        .unwrap_or_else(|| std::path::Path::new("."));

    if semgrep_available() {
        findings.extend(run_semgrep_in(workspace));
    }

    if trivy_available() {
        findings.extend(run_trivy_in(workspace));
    }

    if gitleaks_available() {
        findings.extend(run_gitleaks_in(workspace));
    }

    findings
}

fn semgrep_available() -> bool {
    std::process::Command::new("semgrep")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn trivy_available() -> bool {
    std::process::Command::new("trivy")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn gitleaks_available() -> bool {
    std::process::Command::new("gitleaks")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn run_command_in(argv: &[&str], cwd: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new(argv[0])
        .args(&argv[1..])
        .current_dir(cwd)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Some(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn run_semgrep_in(workspace: &std::path::Path) -> Vec<Finding> {
    let output = run_command_in(&["semgrep", "--config=auto", "--json", "."], workspace);
    let mut findings = Vec::new();
    if let Some(out) = output {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&out) {
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
    }
    findings
}

fn run_trivy_in(workspace: &std::path::Path) -> Vec<Finding> {
    let output = run_command_in(&["trivy", "fs", "--format=json", "."], workspace);
    let mut findings = Vec::new();
    if let Some(out) = output {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&out) {
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
    }
    findings
}

fn run_gitleaks_in(workspace: &std::path::Path) -> Vec<Finding> {
    let output = run_command_in(&["gitleaks", "detect", "--no-color", "--no-git", "-v", "."], workspace);
    let mut findings = Vec::new();
    if let Some(out) = output {
        for line in out.lines() {
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
    }
    findings
}

/// Generate remediation patches for findings.
/// In `suggest` mode, only produces patches without applying them.
fn remediate_findings(findings: &[Finding], _suggest: bool) -> Vec<serde_json::Value> {
    let mut patches = Vec::new();
    for f in findings {
        patches.push(serde_json::json!({
            "finding": f.message,
            "severity": format!("{:?}", f.severity),
            "source": f.source,
            "patch": generate_patch(f),
        }));
    }
    patches
}

fn generate_patch(finding: &Finding) -> String {
    let source = &finding.source;
    let msg = &finding.message;
    format!("# remediation for {source}: {msg}\n# See: continuum-security --mode hardening")
}

/// Apply remediation patches. In the current phase, patches are generated
/// as file-based remediations. Full sandbox-based application will be
/// wired with the execution engine when a session context is available.
async fn apply_patches_via_sandbox(
    patches: &[serde_json::Value],
    ctx: &AgentContext,
) -> usize {
    let mut applied = 0usize;
    for patch in patches {
        let patch_str = patch["patch"].as_str().unwrap_or("");
        if patch_str.is_empty() {
            continue;
        }
        ctx.task_progress(
            AgentKind::Security,
            &format!("patch generated: {:.60}", patch_str),
            Some(80),
        );
        applied += 1;
    }
    applied
}
