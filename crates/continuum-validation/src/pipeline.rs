use crate::validators::AllValidators;
use continuum_core::validator::{ValidationContext, ValidationReport, ValidationTarget, Validator};

/// Result of a full pipeline run.
#[derive(Debug, Default)]
pub struct PipelineResult {
    /// Reports from each stage, in stage order.
    pub reports: Vec<ValidationReport>,
    /// Whether the pipeline passed overall.
    pub passed: bool,
}

/// The 10-stage validation pipeline. Runs validators in order, short-circuiting
/// on required failures.
#[derive(Debug, Default)]
pub struct Pipeline {
    validators: AllValidators,
}

impl Pipeline {
    /// Create a new pipeline with all ten stage validators.
    pub fn new() -> Self {
        Self {
            validators: AllValidators::new(),
        }
    }

    /// Register an additional validator.
    pub fn add(&mut self, v: Box<dyn Validator>) {
        self.validators.add(v);
    }

    /// Run all validators in stage order. Required validators that fail
    /// short-circuit the pipeline. Optional validators run regardless.
    pub async fn run(&self, target: &ValidationTarget) -> PipelineResult {
        let ctx = ValidationContext::new();
        let mut reports = Vec::new();
        let mut passed = true;

        for v in self.validators.iter() {
            let report = match v.run(target, &ctx).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(stage = ?v.stage(), error = %e, "validator failed");
                    ValidationReport::new(v.stage(), vec![], false, 0)
                }
            };

            tracing::info!(
                stage = ?report.stage,
                passed = report.passed,
                findings = report.findings.len(),
                duration = report.duration_ms,
                "validation stage complete"
            );

            if !report.passed {
                passed = false;
                if v.required() {
                    tracing::warn!(stage = ?v.stage(), "required stage failed, short-circuiting");
                    reports.push(report);
                    break;
                }
            }

            reports.push(report);
        }

        PipelineResult { reports, passed }
    }
}
