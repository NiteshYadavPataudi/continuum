//! Sandbox and SandboxHandle traits.
//!
//! Implementations live in `continuum-sandbox`. Docker is the default backend;
//! Firecracker support sits behind a feature flag. Every external tool runs
//! inside a sandbox via [`SandboxHandle::exec`] — there is no host-side
//! `Command::spawn` in any tool crate.

use crate::caps::{Cap, HostExec};
use crate::ids::{SandboxId, SnapshotId};
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Specification for spawning a new sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SandboxSpec {
    /// Container image (Docker reference, e.g. `"continuum/runtime:latest"`).
    pub image: String,
    /// Project directory mounted at `/workspace` inside the sandbox.
    pub workspace: PathBuf,
    /// CPU quota (cores).
    pub cpu_quota: f32,
    /// Memory quota (megabytes).
    pub memory_mb: u32,
    /// Whether outbound network is permitted.
    pub network_egress: bool,
}

/// One process execution inside a sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExecRequest {
    /// Argv. `argv[0]` is the binary.
    pub argv: Vec<String>,
    /// Working directory inside the sandbox.
    pub cwd: Option<PathBuf>,
    /// Extra environment variables.
    pub env: Vec<(String, String)>,
    /// Whether stdin should be left open (default false).
    pub stdin_open: bool,
}

impl ExecRequest {
    /// Create a new exec request.
    pub fn new(argv: Vec<String>) -> Self {
        Self {
            argv,
            cwd: None,
            env: vec![],
            stdin_open: false,
        }
    }
}

/// One chunk of streamed exec output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExecEvent {
    /// Bytes from stdout.
    Stdout(Vec<u8>),
    /// Bytes from stderr.
    Stderr(Vec<u8>),
    /// Process has exited with the given code.
    Exit(i32),
}

/// Stream of exec events. Terminates after the [`ExecEvent::Exit`] frame.
pub type ExecStream = BoxStream<'static, Result<ExecEvent, SandboxError>>;

/// Errors specific to sandbox operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SandboxError {
    /// The backend (Docker daemon, etc.) is unavailable.
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),
    /// The requested image could not be pulled or built.
    #[error("image error: {0}")]
    Image(String),
    /// The sandbox was killed externally (OOM, user, scheduler).
    #[error("sandbox terminated")]
    Terminated,
    /// File I/O against the sandbox failed.
    #[error("fs error: {0}")]
    Fs(String),
    /// Catch-all.
    #[error("sandbox error: {0}")]
    Other(String),
}

/// Creates sandboxes. The associated `Handle` type lets the runtime choose
/// a backend at compile time without `Box<dyn>` indirection.
#[async_trait]
pub trait Sandbox: Send + Sync {
    /// The handle type produced by [`Sandbox::spawn`].
    type Handle: SandboxHandle;

    /// Spawn a new sandbox instance from the given spec.
    async fn spawn(&self, spec: SandboxSpec) -> Result<Self::Handle, SandboxError>;
}

/// Operations on a live sandbox instance.
#[async_trait]
pub trait SandboxHandle: Send + Sync {
    /// Stable ID of this sandbox.
    fn id(&self) -> SandboxId;

    /// Execute a process inside the sandbox, streaming stdout/stderr.
    async fn exec(&self, _cap: &Cap<HostExec>, cmd: ExecRequest) -> Result<ExecStream, SandboxError>;

    /// Write a file inside the sandbox.
    async fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<(), SandboxError>;

    /// Read a file from inside the sandbox.
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, SandboxError>;

    /// Snapshot the sandbox's filesystem state.
    async fn snapshot(&self) -> Result<SnapshotId, SandboxError>;

    /// Restore from a previously taken snapshot.
    async fn restore(&self, snap: SnapshotId) -> Result<(), SandboxError>;

    /// Tear the sandbox down. Consumes the handle.
    async fn shutdown(self: Box<Self>) -> Result<(), SandboxError>;
}
