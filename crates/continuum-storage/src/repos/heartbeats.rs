use sqlx::SqlitePool;

use crate::error::StorageError;

/// Repository for recording session heartbeat events with cost tracking.
pub struct HeartbeatRepo {
    pool: SqlitePool,
}

impl HeartbeatRepo {
    /// Create a new `HeartbeatRepo` backed by the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Record a heartbeat for a session with the current retry count and cumulative cost.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn upsert(
        &self,
        session_id: &str,
        retry_count: i32,
        cumulative_usd: f64,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO session_heartbeats (session_id, recorded_at, retry_count, cumulative_usd) VALUES (?, datetime('now'), ?, ?)",
        )
        .bind(session_id)
        .bind(retry_count)
        .bind(cumulative_usd)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Retrieve the most recent heartbeat for a session.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn latest(&self, session_id: &str) -> Result<Option<HeartbeatRow>, StorageError> {
        let row = sqlx::query_as::<_, HeartbeatRow>(
            "SELECT session_id, recorded_at, retry_count, cumulative_usd FROM session_heartbeats WHERE session_id = ? ORDER BY recorded_at DESC LIMIT 1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }
}

/// A row representing a single session heartbeat.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HeartbeatRow {
    /// The session this heartbeat belongs to.
    pub session_id: String,
    /// ISO-8601 timestamp of when the heartbeat was recorded.
    pub recorded_at: String,
    /// Number of retries performed so far in the session.
    pub retry_count: i32,
    /// Cumulative USD cost for the session so far.
    pub cumulative_usd: f64,
}
