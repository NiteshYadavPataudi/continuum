use sqlx::SqlitePool;

use crate::error::StorageError;

/// Repository for managing memory items (summaries and details) per run.
pub struct MemoryRepo {
    pool: SqlitePool,
}

impl MemoryRepo {
    /// Create a new `MemoryRepo` backed by the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new memory item.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn create(
        &self,
        id: &str,
        run_id: &str,
        layer: &str,
        summary: &str,
        detail: &str,
        token_count: i32,
    ) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO memory_items (id, run_id, layer, summary, detail, token_count) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(id)
            .bind(run_id)
            .bind(layer)
            .bind(summary)
            .bind(detail)
            .bind(token_count)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Retrieve a memory item by its id.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn get(&self, id: &str) -> Result<Option<MemoryRow>, StorageError> {
        let row = sqlx::query_as::<_, MemoryRow>(
            "SELECT id, run_id, layer, summary, detail, token_count, created_at FROM memory_items WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// List all memory items for a run, ordered by creation time.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn list_by_run(&self, run_id: &str) -> Result<Vec<MemoryRow>, StorageError> {
        let rows = sqlx::query_as::<_, MemoryRow>(
            "SELECT id, run_id, layer, summary, detail, token_count, created_at FROM memory_items WHERE run_id = ? ORDER BY created_at ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Delete a single memory item by id. Returns `true` if a row was removed.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn delete(&self, id: &str) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM memory_items WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete all memory items for a run. Returns the number of rows deleted.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn delete_by_run(&self, run_id: &str) -> Result<u64, StorageError> {
        let result = sqlx::query("DELETE FROM memory_items WHERE run_id = ?")
            .bind(run_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

/// A row representing a stored memory item.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MemoryRow {
    /// Unique memory item identifier.
    pub id: String,
    /// The run this memory belongs to.
    pub run_id: String,
    /// The memory layer (e.g. "core", "episodic", "working").
    pub layer: String,
    /// A brief summary of the memory.
    pub summary: String,
    /// The full detail / content of the memory.
    pub detail: String,
    /// Approximate token count for the memory content.
    pub token_count: i32,
    /// ISO-8601 timestamp of when the memory was created.
    pub created_at: String,
}
