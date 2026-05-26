//! Shared types, traits, IDs, errors, and capability tokens for Continuum.
//!
//! Every other crate in the workspace depends on `continuum-core`. This crate
//! has no I/O, no async runtime, and no domain logic — it only defines the
//! contracts that other crates implement.

#![warn(missing_docs)]

pub mod agent;
pub mod caps;
pub mod error;
pub mod ids;
pub mod memory;
pub mod model;
pub mod planner;
pub mod recovery;
pub mod repo;
pub mod sandbox;
pub mod tool;
pub mod validator;

pub use error::{ContinuumError, Result};
pub use ids::{
    AgentId, CheckpointId, MemoryId, ProviderId, RunId, SandboxId, SessionId, SnapshotId, TaskId,
    ToolId,
};

/// Re-export `tokio_util::sync::CancellationToken` so trait implementers don't
/// need a direct dependency on `tokio-util`.
pub use tokio_util::sync::CancellationToken;
