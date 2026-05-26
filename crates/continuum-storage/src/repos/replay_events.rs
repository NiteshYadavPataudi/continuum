use sqlx::SqlitePool;

use crate::error::StorageError;

pub struct ReplayEventRepo {
    pool: SqlitePool,
}

impl ReplayEventRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        session_id: &str,
        target: &str,
        payload: &str,
    ) -> Result<i64, StorageError> {
        let result = sqlx::query(
            "INSERT INTO replay_events (session_id, target, payload) VALUES (?, ?, ?)",
        )
        .bind(session_id)
        .bind(target)
        .bind(payload)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReplayEventRow {
    pub id: i64,
    pub session_id: String,
    pub recorded_at: String,
    pub target: String,
    pub payload: String,
}
