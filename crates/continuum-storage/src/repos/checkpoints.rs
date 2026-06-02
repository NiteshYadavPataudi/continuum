use sqlx::SqlitePool;

use crate::error::StorageError;

/// Repository for managing agent state checkpoints persisted in the database.
pub struct CheckpointRepo {
    pool: SqlitePool,
}

impl CheckpointRepo {
    /// Create a new `CheckpointRepo` backed by the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new checkpoint.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
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

    /// Retrieve a checkpoint by its id.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn get(&self, id: &str) -> Result<Option<CheckpointRow>, StorageError> {
        let row = sqlx::query_as::<_, CheckpointRow>(
            "SELECT id, run_id, snapshot, parent_id, created_at FROM checkpoints WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// List all checkpoints for a given run, ordered by creation time.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
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

/// A row representing a single agent state checkpoint.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CheckpointRow {
    /// Unique checkpoint identifier.
    pub id: String,
    /// The run this checkpoint belongs to.
    pub run_id: String,
    /// Opaque serialised agent state.
    pub snapshot: Vec<u8>,
    /// Optional id of the parent checkpoint (for checkpoint trees).
    pub parent_id: Option<String>,
    /// ISO-8601 timestamp of when the checkpoint was created.
    pub created_at: String,
}
