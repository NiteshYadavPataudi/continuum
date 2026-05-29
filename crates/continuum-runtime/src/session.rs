use std::sync::Arc;

use continuum_core::ids::SessionId;
use continuum_core::memory::MemoryStore;
use continuum_core::recovery::RecoveryStore;
use continuum_core::sandbox::SandboxHandle;
use continuum_core::CancellationToken;
use std::path::PathBuf;

/// One user-driven execution session.
pub struct Session {
    /// Stable identifier for the session.
    pub id: SessionId,
    /// Memory store for the session.
    pub memory: Option<Arc<dyn MemoryStore>>,
    /// Recovery store for checkpoints and replay.
    pub recovery: Option<Arc<dyn RecoveryStore>>,
    /// Sandbox handle for isolated execution.
    pub sandbox: Option<Arc<dyn SandboxHandle>>,
    /// Workspace root used for per-agent worktree isolation.
    pub workspace_root: Option<PathBuf>,
    /// Cancellation token for the session.
    pub cancel: CancellationToken,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("memory", &self.memory.as_ref().map(|_| "<store>"))
            .field("recovery", &self.recovery.as_ref().map(|_| "<store>"))
            .field("sandbox", &self.sandbox.as_ref().map(|_| "<handle>"))
            .finish()
    }
}

impl Session {
    /// Create a new session with a fresh ID.
    pub fn new() -> Self {
        Self {
            id: SessionId::new(),
            memory: None,
            recovery: None,
            sandbox: None,
            workspace_root: None,
            cancel: CancellationToken::new(),
        }
    }

    /// Attach a memory store.
    pub fn with_memory(mut self, memory: Arc<dyn MemoryStore>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Attach a recovery store.
    pub fn with_recovery(mut self, recovery: Arc<dyn RecoveryStore>) -> Self {
        self.recovery = Some(recovery);
        self
    }

    /// Attach a sandbox handle.
    pub fn with_sandbox(mut self, sandbox: Arc<dyn SandboxHandle>) -> Self {
        self.sandbox = Some(sandbox);
        self
    }

    /// Attach a workspace root for isolated worktrees.
    pub fn with_workspace_root(mut self, workspace_root: PathBuf) -> Self {
        self.workspace_root = Some(workspace_root);
        self
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
