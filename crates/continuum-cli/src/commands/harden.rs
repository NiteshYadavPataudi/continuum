use continuum_agents::SecurityAgent;
use continuum_core::agent::{Agent, AgentContext, AgentTask};
use continuum_core::ids::ModelId;
use continuum_core::CancellationToken;
use continuum_security::{HardeningMode, SecurityPolicy};

use super::{CmdResult, HardenArgs};

pub async fn run(args: HardenArgs) -> CmdResult {
    let mode: HardeningMode = args.mode.parse().map_err(|e: String| e)?;
    let project = args.project.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!("── Security Hardening ──");
    println!("  Mode:    {mode:?}");
    println!("  Project: {}", project.display());
    println!();

    let _policy = SecurityPolicy::new(mode);

    // Build the security agent
    let agent = SecurityAgent::new(None, ModelId::new("claude-sonnet-4-20250514"));

    // Prepare the task payload with the policy
    let payload = serde_json::json!({
        "mode": format!("{:?}", mode).to_lowercase(),
        "suggest": false,
        "exit_code": true,
    });
    let task = AgentTask::new(continuum_core::ids::TaskId::new(), payload);

    // Run the agent
    let outcome = agent
        .handle(task, &AgentContext::new(), CancellationToken::new())
        .await
        .map_err(|e| format!("security agent failed: {e}"))?;

    let artifacts = &outcome.artifacts;

    // Print the audit report
    if let Some(report_md) = artifacts.get("report_md").and_then(|v| v.as_str()) {
        let report_path = project.join(".continuum").join("reports");
        tokio::fs::create_dir_all(&report_path).await?;
        let report_file = report_path.join("AUDIT_REPORT.md");
        tokio::fs::write(&report_file, report_md).await?;
        println!("  Audit report written to: {}", report_file.display());
    }

    // Print patches in hardening mode
    if let Some(patches) = artifacts.get("patches").and_then(|v| v.as_array()) {
        if !patches.is_empty() {
            println!();
            println!("  --- Remediation patches ---");
            for (i, patch) in patches.iter().enumerate() {
                let finding = patch.get("finding").and_then(|v| v.as_str()).unwrap_or("?");
                let severity = patch.get("severity").and_then(|v| v.as_str()).unwrap_or("?");
                println!("  {}. [{severity}] {finding}", i + 1);
            }
        }
    }

    // Print compliance attestation in enterprise mode
    if mode == HardeningMode::Enterprise {
        if let Some(compliance_md) = artifacts.get("compliance_md").and_then(|v| v.as_str()) {
            let compliance_path = project.join(".continuum").join("reports");
            let compliance_file = compliance_path.join("COMPLIANCE.md");
            tokio::fs::write(&compliance_file, compliance_md).await?;
            println!();
            println!("  Compliance attestation written to: {}", compliance_file.display());
        }
    }

    // Handle exit code
    if let Some(exit_code) = artifacts.get("exit_code").and_then(|v| v.as_i64()) {
        if exit_code != 0 {
            println!();
            println!("  ⚠  Findings detected above severity floor. Exiting with code {exit_code}.");
            std::process::exit(exit_code as i32);
        }
    }

    println!();
    println!("  ✓ Hardening pass complete.");
    Ok(())
}
