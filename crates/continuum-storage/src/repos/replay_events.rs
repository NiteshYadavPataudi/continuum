use sqlx::SqlitePool;

use crate::error::StorageError;

/// Repository for recording and replaying session events.
pub struct ReplayEventRepo {
    pool: SqlitePool,
}

impl ReplayEventRepo {
    /// Create a new `ReplayEventRepo` backed by the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Record a new replay event and return its auto-generated id.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn create(
        &self,
        session_id: &str,
        target: &str,
        payload: &str,
    ) -> Result<i64, StorageError> {
        let result =
            sqlx::query("INSERT INTO replay_events (session_id, target, payload) VALUES (?, ?, ?)")
                .bind(session_id)
                .bind(target)
                .bind(payload)
                .execute(&self.pool)
                .await?;
        Ok(result.last_insert_rowid())
    }

    /// List replay events for a session, optionally starting after a given event id.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn list_by_session(
        &self,
        session_id: &str,
        after_id: Option<i64>,
    ) -> Result<Vec<ReplayEventRow>, StorageError> {
        let rows = if let Some(after) = after_id {
            sqlx::query_as::<_, ReplayEventRow>(
                "SELECT id, session_id, recorded_at, target, payload FROM replay_events WHERE session_id = ? AND id > ? ORDER BY id ASC",
            )
            .bind(session_id)
            .bind(after)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ReplayEventRow>(
                "SELECT id, session_id, recorded_at, target, payload FROM replay_events WHERE session_id = ? ORDER BY id ASC",
            )
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }
}

/// A row representing a single recorded replay event.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReplayEventRow {
    /// Auto-generated primary key.
    pub id: i64,
    /// The session this event belongs to.
    pub session_id: String,
    /// ISO-8601 timestamp of when the event was recorded.
    pub recorded_at: String,
    /// The target component or handler for the event.
    pub target: String,
    /// The JSON payload of the event.
    pub payload: String,
}
