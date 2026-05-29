//! Recovery, checkpoint, and replay traits.
//!
//! `continuum-recovery` writes a checkpoint after every completed task node.
//! `continuum resume` and `continuum replay` walk the checkpoint chain;
//! `detect_stuck` samples scheduler heartbeats to short-circuit infinite loops.

use crate::ids::{CheckpointId, SessionId};
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// Opaque session state snapshotted at each checkpoint.
///
/// Phase 1 keeps this as a JSON blob; phase 6 promotes it to a typed
/// `SessionState` once the runtime's data model stabilises.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SessionState {
    /// Free-form payload (typically the runtime's serialized state).
    pub payload: serde_json::Value,
}

impl SessionState {
    /// Create a new session state with the given payload.
    pub fn new(payload: serde_json::Value) -> Self {
        Self { payload }
    }
}

/// A persisted checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Checkpoint {
    /// Stable ID.
    pub id: CheckpointId,
    /// Session this checkpoint belongs to.
    pub session: SessionId,
    /// When the checkpoint was taken.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// State at the time of checkpointing.
    pub state: SessionState,
}

impl Checkpoint {
    /// Create a new checkpoint.
    pub fn new(
        id: CheckpointId,
        session: SessionId,
        created_at: OffsetDateTime,
        state: SessionState,
    ) -> Self {
        Self {
            id,
            session,
            created_at,
            state,
        }
    }
}

/// One event in a replay stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReplayEvent {
    /// Wall-clock time at which the event was originally observed.
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    /// Event payload (matches the runtime's tracing event schema).
    pub payload: serde_json::Value,
}

impl ReplayEvent {
    /// Create a new replay event.
    pub fn new(at: OffsetDateTime, payload: serde_json::Value) -> Self {
        Self { at, payload }
    }
}

/// Stream of replay events, ordered oldest-to-newest.
pub type ReplayStream = BoxStream<'static, Result<ReplayEvent, RecoveryError>>;

/// Why a session might be flagged as stuck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StuckReason {
    /// No scheduler heartbeat for too long.
    NoHeartbeat,
    /// Same task has been retried too many times.
    RetryLoop,
    /// Token / cost budget overrun.
    BudgetOverrun,
}

/// Stuck-task / infinite-loop detector signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StuckSignal {
    /// Why the session was flagged.
    pub reason: StuckReason,
    /// Free-form detail.
    pub detail: String,
}

impl StuckSignal {
    /// Create a new stuck signal.
    pub fn new(reason: StuckReason, detail: impl Into<String>) -> Self {
        Self {
            reason,
            detail: detail.into(),
        }
    }
}

/// Errors specific to recovery.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RecoveryError {
    /// Storage layer failure.
    #[error("storage: {0}")]
    Storage(String),
    /// The requested checkpoint does not exist.
    #[error("checkpoint not found")]
    NotFound,
    /// Catch-all.
    #[error("recovery error: {0}")]
    Other(String),
}

/// Persistent recovery contract.
#[async_trait]
pub trait RecoveryStore: Send + Sync {
    /// Take a checkpoint of the current session state.
    async fn checkpoint(
        &self,
        session: SessionId,
        state: &SessionState,
    ) -> Result<CheckpointId, RecoveryError>;

    /// Fetch the most recent checkpoint for `session`, if any.
    async fn latest(&self, session: SessionId) -> Result<Option<Checkpoint>, RecoveryError>;

    /// Roll the runtime back to the named checkpoint.
    async fn rollback(&self, to: CheckpointId) -> Result<SessionState, RecoveryError>;

    /// Stream the replay events for `session`, starting from `from`.
    async fn replay(
        &self,
        session: SessionId,
        from: CheckpointId,
    ) -> Result<ReplayStream, RecoveryError>;

    /// Sample the heartbeat detector. Returns `Some` if the session looks stuck.
    async fn detect_stuck(&self, session: SessionId) -> Result<Option<StuckSignal>, RecoveryError>;
}
