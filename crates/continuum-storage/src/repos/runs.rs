use sqlx::SqlitePool;

use crate::error::StorageError;

/// Repository for managing agent runs (top-level execution sessions).
pub struct RunRepo {
    pool: SqlitePool,
}

impl RunRepo {
    /// Create a new `RunRepo` backed by the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new run with a given id and goal.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn create(&self, id: &str, goal: &str) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO runs (id, goal) VALUES (?, ?)")
            .bind(id)
            .bind(goal)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Retrieve a run by its id.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn get(&self, id: &str) -> Result<Option<RunRow>, StorageError> {
        let row = sqlx::query_as::<_, RunRow>(
            "SELECT id, goal, status, created_at, updated_at FROM runs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// List all runs, ordered by creation time descending (most recent first).
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn list(&self) -> Result<Vec<RunRow>, StorageError> {
        let rows = sqlx::query_as::<_, RunRow>(
            "SELECT id, goal, status, created_at, updated_at FROM runs ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

/// A row representing a single run.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RunRow {
    /// Unique run identifier.
    pub id: String,
    /// The high-level goal of the run.
    pub goal: String,
    /// Current status (e.g. "active", "completed", "failed").
    pub status: String,
    /// ISO-8601 timestamp of when the run was created.
    pub created_at: String,
    /// ISO-8601 timestamp of the last update to the run.
    pub updated_at: String,
}
