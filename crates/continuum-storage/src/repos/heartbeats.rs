use sqlx::SqlitePool;

use crate::error::StorageError;

pub struct HeartbeatRepo {
    pool: SqlitePool,
}

impl HeartbeatRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HeartbeatRow {
    pub session_id: String,
    pub recorded_at: String,
    pub retry_count: i32,
    pub cumulative_usd: f64,
}
