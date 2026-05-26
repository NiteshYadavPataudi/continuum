use sqlx::SqlitePool;

use crate::error::StorageError;

pub struct RunRepo {
    pool: SqlitePool,
}

impl RunRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, id: &str, goal: &str) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO runs (id, goal) VALUES (?, ?)")
            .bind(id)
            .bind(goal)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Result<Option<RunRow>, StorageError> {
        let row = sqlx::query_as::<_, RunRow>(
            "SELECT id, goal, status, created_at, updated_at FROM runs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list(&self) -> Result<Vec<RunRow>, StorageError> {
        let rows = sqlx::query_as::<_, RunRow>(
            "SELECT id, goal, status, created_at, updated_at FROM runs ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RunRow {
    pub id: String,
    pub goal: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
