use std::collections::BTreeMap;

/// SOC2-style control mapping for enterprise compliance attestation.
#[derive(Debug, Clone, Default)]
pub struct ComplianceAttestation {
    /// Controls mapped by canonical name.
    pub controls: BTreeMap<String, ControlStatus>,
}

/// Status of one security control.
#[derive(Debug, Clone)]
pub struct ControlStatus {
    /// Human-readable description of the control.
    pub description: String,
    /// Whether the control is met.
    pub status: ControlState,
    /// Code path, config, or doc that satisfies this control.
    pub citation: String,
}

/// One of `met`, `not-met`, or `n/a`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlState {
    /// Control requirement is satisfied.
    Met,
    /// Control requirement is not satisfied.
    NotMet,
    /// Not applicable to this project.
    Na,
}

impl ComplianceAttestation {
    /// Build the standard SOC2 control map.
    pub fn build() -> Self {
        let mut controls = BTreeMap::new();

        // Access control
        controls.insert(
            "CC6.1".into(),
            ControlStatus {
                description: "Logical and physical access to information assets is restricted.".into(),
                status: ControlState::Met,
                citation: ".continuum/config + sandbox network=none for audit mode".into(),
            },
        );
        controls.insert(
            "CC6.2".into(),
            ControlStatus {
                description: "Users are provisioned, authenticated, and de-provisioned.".into(),
                status: ControlState::Met,
                citation: "Capability token system (continuum-core::caps) gates API key access".into(),
            },
        );

        // Change management
        controls.insert(
            "CC8.1".into(),
            ControlStatus {
                description: "Changes to infrastructure, data, and software are authorised, tested, and tracked.".into(),
                status: ControlState::Met,
                citation: "Validation pipeline runs before every merge; hardening loop re-runs scanners".into(),
            },
        );

        // Risk mitigation
        controls.insert(
            "CC7.1".into(),
            ControlStatus {
                description: "Vulnerabilities are identified, monitored, and remediated.".into(),
                status: ControlState::Met,
                citation: "Security audit mode runs Semgrep + Trivy + Gitleaks; findings are surfaced in AUDIT_REPORT.md".into(),
            },
        );
        controls.insert(
            "CC7.2".into(),
            ControlStatus {
                description: "Security incidents are detected and responded to.".into(),
                status: ControlState::NotMet,
                citation: "Continuum does not currently implement automated incident response".into(),
            },
        );

        // Availability
        controls.insert(
            "A1.2".into(),
            ControlStatus {
                description: "System components are monitored and alert on anomalies.".into(),
                status: ControlState::Met,
                citation: "Prometheus metrics + heartbeat-based stuck session detection".into(),
            },
        );

        Self { controls }
    }

    /// Render as a Markdown document suitable for `COMPLIANCE.md`.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Compliance Attestation\n\n");
        md.push_str("This document maps Continuum's security controls to SOC2 criteria.\n\n");
        md.push_str("> **Disclaimer:** This is a self-assessment. It is not a certified SOC2 report.\n\n");
        md.push_str("| Control | Description | Status | Citation |\n");
        md.push_str("|---------|-------------|--------|----------|\n");
        for (id, ctrl) in &self.controls {
            let status_str = match ctrl.status {
                ControlState::Met => "✅ met",
                ControlState::NotMet => "❌ not-met",
                ControlState::Na => "— n/a",
            };
            md.push_str(&format!("| **{id}** | {} | {status_str} | `{}` |\n", ctrl.description, ctrl.citation));
        }
        md.push('\n');
        md
    }
}
