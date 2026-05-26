//! Validator trait and pipeline stages.
//!
//! The 10-stage validation pipeline lives in `continuum-validation`. Each
//! stage may have multiple validators (e.g. lint stage runs Biome + Clippy).
//! Required validators short-circuit on failure; optional validators run in
//! parallel and contribute findings without blocking the pipeline.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

use crate::sandbox::SandboxHandle;

/// The ten stages of the validation pipeline, ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationStage {
    /// Source compiles.
    Compile,
    /// Linter clean.
    Lint,
    /// Type checker clean.
    TypeCheck,
    /// Unit tests pass.
    UnitTest,
    /// Integration tests pass.
    IntegrationTest,
    /// End-to-end tests pass (browser + API surfaces).
    E2eTest,
    /// Security scanners clean.
    SecurityScan,
    /// Application starts.
    Startup,
    /// Performance budget met.
    Performance,
    /// No regressions vs. baseline.
    Regression,
}

/// What is being validated.
#[derive(Clone)]
#[non_exhaustive]
pub struct ValidationTarget {
    /// Workspace root inside the sandbox.
    pub workspace: PathBuf,
    /// Subset of files in scope (empty = whole workspace).
    pub paths: Vec<PathBuf>,
    /// Sandbox handle for executing tools.
    pub sandbox: Option<Arc<dyn SandboxHandle>>,
}

impl std::fmt::Debug for ValidationTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationTarget")
            .field("workspace", &self.workspace)
            .field("paths", &self.paths)
            .field("sandbox", &self.sandbox.as_ref().map(|_| "<sandbox>"))
            .finish()
    }
}

impl ValidationTarget {
    /// Create a new validation target.
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            paths: vec![],
            sandbox: None,
        }
    }

    /// Set the sandbox handle.
    pub fn with_sandbox(mut self, sandbox: Arc<dyn SandboxHandle>) -> Self {
        self.sandbox = Some(sandbox);
        self
    }

    /// Set the paths to validate.
    pub fn with_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.paths = paths;
        self
    }
}

/// Per-validator runtime handles.
#[derive(Debug, Clone)]
pub struct ValidationContext;

impl ValidationContext {
    /// Create a new validation context.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Informational, no action required.
    Info,
    /// Warning, may be ignored but should be reviewed.
    Warning,
    /// Error, blocks validation if the stage is required.
    Error,
    /// Critical security or correctness issue.
    Critical,
}

/// A single finding produced by a validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Finding {
    /// Originating tool/validator.
    pub source: String,
    /// Severity.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// File path the finding applies to, if any.
    pub file: Option<PathBuf>,
    /// 1-indexed line number, if known.
    pub line: Option<u32>,
}

impl Finding {
    /// Create a new finding.
    pub fn new(
        source: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
        file: Option<PathBuf>,
        line: Option<u32>,
    ) -> Self {
        Self {
            source: source.into(),
            severity,
            message: message.into(),
            file,
            line,
        }
    }
}

/// Aggregate result of one validator run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ValidationReport {
    /// Stage this report belongs to.
    pub stage: ValidationStage,
    /// Findings discovered.
    pub findings: Vec<Finding>,
    /// `true` if the validator considers the stage passing.
    pub passed: bool,
    /// Wall-clock duration of the validator in milliseconds.
    pub duration_ms: u64,
}

impl ValidationReport {
    /// Create a new validation report.
    pub fn new(
        stage: ValidationStage,
        findings: Vec<Finding>,
        passed: bool,
        duration_ms: u64,
    ) -> Self {
        Self {
            stage,
            findings,
            passed,
            duration_ms,
        }
    }
}

/// Errors specific to validation.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ValidationError {
    /// The underlying tool could not be invoked.
    #[error("tool unavailable: {0}")]
    ToolUnavailable(String),
    /// The tool produced output that could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),
    /// Catch-all.
    #[error("validation error: {0}")]
    Other(String),
}

/// One validator implementation. Multiple validators may target the same
/// [`ValidationStage`]; the pipeline aggregates their reports.
#[async_trait]
pub trait Validator: Send + Sync {
    /// Which stage this validator contributes to.
    fn stage(&self) -> ValidationStage;

    /// Whether the pipeline should short-circuit if this validator fails.
    fn required(&self) -> bool;

    /// Run the validator. Implementations should be cancellation-aware.
    async fn run(
        &self,
        target: &ValidationTarget,
        ctx: &ValidationContext,
    ) -> Result<ValidationReport, ValidationError>;
}
