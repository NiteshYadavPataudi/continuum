use sqlx::SqlitePool;

use crate::error::StorageError;

pub struct CheckpointRepo {
    pool: SqlitePool,
}

impl CheckpointRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        id: &str,
        run_id: &str,
        snapshot: &[u8],
        parent_id: Option<&str>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO checkpoints (id, run_id, snapshot, parent_id) VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(run_id)
        .bind(snapshot)
        .bind(parent_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Result<Option<CheckpointRow>, StorageError> {
        let row = sqlx::query_as::<_, CheckpointRow>(
            "SELECT id, run_id, snapshot, parent_id, created_at FROM checkpoints WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_by_run(&self, run_id: &str) -> Result<Vec<CheckpointRow>, StorageError> {
        let rows = sqlx::query_as::<_, CheckpointRow>(
            "SELECT id, run_id, snapshot, parent_id, created_at FROM checkpoints WHERE run_id = ? ORDER BY created_at ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CheckpointRow {
    pub id: String,
    pub run_id: String,
    pub snapshot: Vec<u8>,
    pub parent_id: Option<String>,
    pub created_at: String,
}
