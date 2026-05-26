//! Error types for Continuum.
//!
//! Every crate defines its own narrow error enum (e.g. `AgentError`,
//! `ModelError`) using `thiserror`. They all `From`-convert into the
//! top-level [`ContinuumError`] so binaries can bubble a single error type.

use thiserror::Error;

/// Top-level Continuum error. Domain crates convert their own error types
/// into this via `From` impls declared alongside the trait definitions.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContinuumError {
    /// An agent failed during task execution.
    #[error("agent error: {0}")]
    Agent(#[from] crate::agent::AgentError),

    /// A model provider call failed.
    #[error("model error: {0}")]
    Model(#[from] crate::model::ModelError),

    /// The sandbox runtime failed.
    #[error("sandbox error: {0}")]
    Sandbox(#[from] crate::sandbox::SandboxError),

    /// A validator stage failed.
    #[error("validation error: {0}")]
    Validation(#[from] crate::validator::ValidationError),

    /// Memory subsystem failure.
    #[error("memory error: {0}")]
    Memory(#[from] crate::memory::MemoryError),

    /// Repository intelligence failure.
    #[error("repo error: {0}")]
    Repo(#[from] crate::repo::RepoError),

    /// Planning engine failure.
    #[error("planner error: {0}")]
    Planner(#[from] crate::planner::PlanError),

    /// Tool runner failure.
    #[error("tool error: {0}")]
    Tool(#[from] crate::tool::ToolError),

    /// Recovery / checkpoint failure.
    #[error("recovery error: {0}")]
    Recovery(#[from] crate::recovery::RecoveryError),

    /// Vector index failure.
    #[error("vector error: {0}")]
    Vector(#[from] crate::memory::VectorError),

    /// Generic configuration error.
    #[error("config error: {0}")]
    Config(String),

    /// Catch-all for unexpected I/O or runtime errors.
    #[error("other: {0}")]
    Other(String),
}

/// Convenience alias for `Result<T, ContinuumError>`.
pub type Result<T, E = ContinuumError> = std::result::Result<T, E>;
