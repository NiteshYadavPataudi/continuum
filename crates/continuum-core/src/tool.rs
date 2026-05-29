//! Tool runner trait.
//!
//! Every external tool (Biome, Trivy, Playwright, etc.) implements
//! [`ToolRunner`]. Tools spawn processes **only** through
//! [`crate::sandbox::SandboxHandle::exec`] — there is no host-side
//! `Command::spawn` anywhere in the tool crates.

use crate::{ids::ToolId, sandbox::SandboxHandle, validator::Finding};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

/// Coarse classification of a tool. Drives which agent picks it up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolFamily {
    /// Linters (Biome, ESLint, Ruff, Clippy).
    Linter,
    /// Formatters (Biome, Prettier, rustfmt).
    Formatter,
    /// Security scanners (OWASP ZAP, Semgrep, Trivy, Gitleaks).
    Security,
    /// Test runners (Vitest, Jest, Pytest, cargo test, k6).
    Test,
    /// Browser drivers (Playwright, chromiumoxide).
    Browser,
    /// Build systems (cargo, npm, pnpm, uv).
    Build,
    /// Container runtimes (Docker).
    Container,
}

/// A rough classification of the project being acted on.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProjectKind {
    /// Primary language (e.g. `"rust"`, `"typescript"`, `"python"`).
    pub language: String,
    /// Package manager (e.g. `"cargo"`, `"pnpm"`).
    pub package_manager: Option<String>,
}

/// One tool invocation.
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ToolInvocation {
    /// Workspace root inside the sandbox.
    pub workspace: PathBuf,
    /// Optional subset of files to target.
    pub paths: Vec<PathBuf>,
    /// Tool-specific arguments serialized as JSON.
    pub args: serde_json::Value,
    /// Optional live event sink.
    #[serde(skip)]
    pub progress: Option<Arc<dyn crate::agent::ExecutionEventSink>>,
    /// Optional task identifier associated with this invocation.
    #[serde(skip)]
    pub task_id: Option<crate::ids::TaskId>,
    /// Optional task label for reporting.
    #[serde(skip)]
    pub task_label: Option<String>,
}

impl std::fmt::Debug for ToolInvocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolInvocation")
            .field("workspace", &self.workspace)
            .field("paths", &self.paths)
            .field("args", &self.args)
            .finish_non_exhaustive()
    }
}

impl ToolInvocation {
    /// Create a new tool invocation.
    pub fn new(workspace: PathBuf, args: serde_json::Value) -> Self {
        Self {
            workspace,
            paths: vec![],
            args,
            progress: None,
            task_id: None,
            task_label: None,
        }
    }

    /// Set the target paths.
    pub fn with_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.paths = paths;
        self
    }

    /// Attach a live event sink.
    pub fn with_progress(
        mut self,
        progress: Arc<dyn crate::agent::ExecutionEventSink>,
        task_id: crate::ids::TaskId,
        task_label: impl Into<String>,
    ) -> Self {
        self.progress = Some(progress);
        self.task_id = Some(task_id);
        self.task_label = Some(task_label.into());
        self
    }

    /// Emit a tool progress event if a reporter is attached.
    pub fn tool_progress(&self, message: impl Into<String>, percent: Option<u8>) {
        if let (Some(progress), Some(task_id)) = (&self.progress, self.task_id) {
            progress.tool_progress(
                task_id,
                self.task_label.clone().unwrap_or_else(|| "tool".into()),
                message.into(),
                percent,
            );
        }
    }

    /// Emit a tool started event if a reporter is attached.
    pub fn tool_started(&self, message: impl Into<String>, percent: Option<u8>) {
        if let (Some(progress), Some(task_id)) = (&self.progress, self.task_id) {
            progress.tool_started(
                task_id,
                self.task_label.clone().unwrap_or_else(|| "tool".into()),
                message.into(),
                percent,
            );
        }
    }

    /// Emit a tool completed event if a reporter is attached.
    pub fn tool_completed(&self, message: impl Into<String>) {
        if let (Some(progress), Some(task_id)) = (&self.progress, self.task_id) {
            progress.tool_completed(
                task_id,
                self.task_label.clone().unwrap_or_else(|| "tool".into()),
                message.into(),
            );
        }
    }

    /// Emit a tool failed event if a reporter is attached.
    pub fn tool_failed(&self, error: impl Into<String>) {
        if let (Some(progress), Some(task_id)) = (&self.progress, self.task_id) {
            progress.tool_failed(
                task_id,
                self.task_label.clone().unwrap_or_else(|| "tool".into()),
                error.into(),
            );
        }
    }
}

/// Normalized result of a tool run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ToolReport {
    /// The tool that produced this report.
    pub tool: ToolId,
    /// Findings (mapped to a uniform shape via [`Finding`]).
    pub findings: Vec<Finding>,
    /// Exit code of the underlying process.
    pub exit_code: i32,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

impl ToolReport {
    /// Create a new tool report.
    pub fn new(tool: ToolId, findings: Vec<Finding>, exit_code: i32, duration_ms: u64) -> Self {
        Self {
            tool,
            findings,
            exit_code,
            duration_ms,
        }
    }
}

/// Errors specific to tools.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ToolError {
    /// The tool's binary could not be found or installed.
    #[error("binary unavailable: {0}")]
    Unavailable(String),
    /// The tool exited with an unexpected status.
    #[error("unexpected exit {code}: {stderr}")]
    BadExit {
        /// Exit code.
        code: i32,
        /// Tail of stderr.
        stderr: String,
    },
    /// Could not parse the tool's output.
    #[error("parse: {0}")]
    Parse(String),
    /// Sandbox-side I/O failure.
    #[error("sandbox: {0}")]
    Sandbox(String),
    /// Catch-all.
    #[error("tool error: {0}")]
    Other(String),
}

/// Contract every tool adapter implements.
#[async_trait]
pub trait ToolRunner: Send + Sync {
    /// Stable tool ID.
    fn id(&self) -> ToolId;

    /// Family this tool belongs to.
    fn family(&self) -> ToolFamily;

    /// Whether this tool can act on the given project.
    fn supports(&self, project: &ProjectKind) -> bool;

    /// Run the tool inside the given sandbox handle.
    async fn run(
        &self,
        invocation: ToolInvocation,
        sandbox: &dyn SandboxHandle,
    ) -> Result<ToolReport, ToolError>;
}
