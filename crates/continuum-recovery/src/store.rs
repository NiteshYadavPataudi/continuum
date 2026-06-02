use std::sync::Arc;

use async_trait::async_trait;
use time::OffsetDateTime;

use continuum_core::ids::{CheckpointId, SessionId};
use continuum_core::recovery::{
    Checkpoint, RecoveryError, RecoveryStore, ReplayEvent, ReplayStream, SessionState, StuckReason,
    StuckSignal,
};
use continuum_storage::VectorBackend;
use continuum_storage::{CheckpointRepo, HeartbeatRepo, MemoryRepo, ReplayEventRepo};

/// SQLite-backed recovery store: checkpoints, heartbeats, and replay events.
pub struct SqliteRecovery {
    checkpoints: Arc<CheckpointRepo>,
    heartbeats: Arc<HeartbeatRepo>,
    events: Arc<ReplayEventRepo>,
    #[allow(dead_code)]
    memory: Arc<MemoryRepo>,
    #[allow(dead_code)]
    vector: Arc<VectorBackend>,
}

impl SqliteRecovery {
    /// Create a new recovery store from pre-initialized repository handles.
    pub fn new(
        checkpoints: CheckpointRepo,
        heartbeats: HeartbeatRepo,
        events: ReplayEventRepo,
        memory: MemoryRepo,
        vector: VectorBackend,
    ) -> Self {
        Self {
            checkpoints: Arc::new(checkpoints),
            heartbeats: Arc::new(heartbeats),
            events: Arc::new(events),
            memory: Arc::new(memory),
            vector: Arc::new(vector),
        }
    }

    /// Record a heartbeat for the session (used by stuck detection).
    pub async fn record_heartbeat(
        &self,
        session: SessionId,
        retry_count: i32,
        cumulative_usd: f64,
    ) -> Result<(), RecoveryError> {
        self.heartbeats
            .upsert(&session.to_string(), retry_count, cumulative_usd)
            .await
            .map_err(|e| RecoveryError::Storage(e.to_string()))
    }
}

#[async_trait]
impl RecoveryStore for SqliteRecovery {
    async fn checkpoint(
        &self,
        session: SessionId,
        state: &SessionState,
    ) -> Result<CheckpointId, RecoveryError> {
        let id = CheckpointId::new();
        let snapshot = serde_json::to_vec(&state)
            .map_err(|e| RecoveryError::Other(format!("serialize state: {e}")))?;

        let run_id = session.to_string();
        self.checkpoints
            .create(&id.to_string(), &run_id, &snapshot, None)
            .await
            .map_err(|e| RecoveryError::Storage(e.to_string()))?;

        Ok(id)
    }

    async fn latest(&self, session: SessionId) -> Result<Option<Checkpoint>, RecoveryError> {
        let run_id = session.to_string();
        let rows = self
            .checkpoints
            .list_by_run(&run_id)
            .await
            .map_err(|e| RecoveryError::Storage(e.to_string()))?;

        let row = match rows.into_iter().last() {
            Some(r) => r,
            None => return Ok(None),
        };

        let state: SessionState = serde_json::from_slice(&row.snapshot)
            .map_err(|e| RecoveryError::Other(format!("deserialize state: {e}")))?;

        let ckpt_id =
            CheckpointId::from(uuid::Uuid::parse_str(&row.id).unwrap_or(uuid::Uuid::nil()));
        let sess_id =
            SessionId::from(uuid::Uuid::parse_str(&row.run_id).unwrap_or(uuid::Uuid::nil()));

        Ok(Some(Checkpoint::new(
            ckpt_id,
            sess_id,
            OffsetDateTime::now_utc(),
            state,
        )))
    }

    async fn rollback(&self, to: CheckpointId) -> Result<SessionState, RecoveryError> {
        let row = self
            .checkpoints
            .get(&to.to_string())
            .await
            .map_err(|e| RecoveryError::Storage(e.to_string()))?
            .ok_or(RecoveryError::NotFound)?;

        let state: SessionState = serde_json::from_slice(&row.snapshot)
            .map_err(|e| RecoveryError::Other(format!("deserialize state: {e}")))?;

        Ok(state)
    }

    async fn replay(
        &self,
        session: SessionId,
        from: CheckpointId,
    ) -> Result<ReplayStream, RecoveryError> {
        let session_id = session.to_string();
        let events = self.events.clone();
        let from_id = from.to_string().parse::<i64>().unwrap_or(0);
        let rows = events
            .list_by_session(&session_id, Some(from_id))
            .await
            .unwrap_or_default();
        let items: Vec<Result<ReplayEvent, RecoveryError>> = rows
            .into_iter()
            .map(|row| {
                let payload: serde_json::Value =
                    serde_json::from_str(&row.payload).unwrap_or(serde_json::Value::Null);
                Ok(ReplayEvent::new(OffsetDateTime::now_utc(), payload))
            })
            .collect();
        Ok(Box::pin(futures::stream::iter(items)))
    }

    async fn detect_stuck(&self, session: SessionId) -> Result<Option<StuckSignal>, RecoveryError> {
        let hb = self
            .heartbeats
            .latest(&session.to_string())
            .await
            .map_err(|e| RecoveryError::Storage(e.to_string()))?;

        match hb {
            None => Ok(Some(StuckSignal::new(
                StuckReason::NoHeartbeat,
                "no heartbeat ever recorded",
            ))),
            Some(row) => {
                if row.retry_count > 5 {
                    return Ok(Some(StuckSignal::new(
                        StuckReason::RetryLoop,
                        format!("retried {} times", row.retry_count),
                    )));
                }
                Ok(None)
            }
        }
    }
}
