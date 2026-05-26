use std::fmt::Write;

use continuum_core::validator::{Finding, Severity};

/// Structured representation of an audit report.
#[derive(Debug, Clone, Default)]
pub struct AuditReport {
    /// Timestamp (Unix millis) when the report was generated.
    pub generated_at_ms: i64,
    /// Hardening mode.
    pub mode: String,
    /// All findings aggregated across tool runs.
    pub findings: Vec<Finding>,
}

impl AuditReport {
    /// Render the report as a Markdown document suitable for `AUDIT_REPORT.md`.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        let _ = writeln!(md, "# Security Audit Report");
        let _ = writeln!(md);
        let _ = writeln!(md, "- **Mode:** {}", self.mode);
        let _ = writeln!(md, "- **Generated:** <code>{}</code>", self.generated_at_ms);
        let _ = writeln!(md);

        if self.findings.is_empty() {
            let _ = writeln!(md, "## Results\n\nNo findings detected.");
            return md;
        }

        for (i, f) in self.findings.iter().enumerate() {
            let severity_label = match f.severity {
                Severity::Critical => "CRITICAL",
                Severity::Error => "ERROR",
                Severity::Warning => "WARNING",
                Severity::Info => "INFO",
            };
            let _ = writeln!(md, "### {}. {}", i + 1, f.message);
            let _ = writeln!(md);
            let _ = writeln!(md, "- **Severity:** {severity_label}");
            let _ = writeln!(md, "- **Source:** {}", f.source);
            if let Some(ref file) = f.file {
                let _ = writeln!(md, "- **File:** `{}`", file.display());
            }
            if let Some(line) = f.line {
                let _ = writeln!(md, "- **Line:** {line}");
            }
            let _ = writeln!(md);
        }

        md
    }
}

/// Builder for `AuditReport`.
#[derive(Debug, Default)]
pub struct AuditReportBuilder {
    mode: String,
    findings: Vec<Finding>,
}

impl AuditReportBuilder {
    /// Create a new builder for the given hardening mode.
    pub fn new(mode: &str) -> Self {
        Self {
            mode: mode.to_string(),
            findings: Vec::new(),
        }
    }

    /// Add a batch of findings.
    pub fn add_findings(&mut self, findings: impl IntoIterator<Item = Finding>) {
        self.findings.extend(findings);
    }

    /// Build the final report.
    pub fn build(self) -> AuditReport {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        AuditReport {
            generated_at_ms: now,
            mode: self.mode,
            findings: self.findings,
        }
    }
}
