use sqlx::SqlitePool;

use crate::error::StorageError;

pub struct MemoryRepo {
    pool: SqlitePool,
}

impl MemoryRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

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

    pub async fn get(&self, id: &str) -> Result<Option<MemoryRow>, StorageError> {
        let row = sqlx::query_as::<_, MemoryRow>(
            "SELECT id, run_id, layer, summary, detail, token_count, created_at FROM memory_items WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_by_run(&self, run_id: &str) -> Result<Vec<MemoryRow>, StorageError> {
        let rows = sqlx::query_as::<_, MemoryRow>(
            "SELECT id, run_id, layer, summary, detail, token_count, created_at FROM memory_items WHERE run_id = ? ORDER BY created_at ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MemoryRow {
    pub id: String,
    pub run_id: String,
    pub layer: String,
    pub summary: String,
    pub detail: String,
    pub token_count: i32,
    pub created_at: String,
}
