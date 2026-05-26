use sqlx::SqlitePool;

use crate::error::StorageError;

pub struct TaskRepo {
    pool: SqlitePool,
}

impl TaskRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        id: &str,
        run_id: &str,
        agent_kind: &str,
        description: &str,
        parent_task_id: Option<&str>,
    ) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO tasks (id, run_id, agent_kind, description, parent_task_id) VALUES (?, ?, ?, ?, ?)")
            .bind(id)
            .bind(run_id)
            .bind(agent_kind)
            .bind(description)
            .bind(parent_task_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Result<Option<TaskRow>, StorageError> {
        let row = sqlx::query_as::<_, TaskRow>(
            "SELECT id, run_id, agent_kind, description, status, parent_task_id, created_at FROM tasks WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_by_run(&self, run_id: &str) -> Result<Vec<TaskRow>, StorageError> {
        let rows = sqlx::query_as::<_, TaskRow>(
            "SELECT id, run_id, agent_kind, description, status, parent_task_id, created_at FROM tasks WHERE run_id = ? ORDER BY created_at ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TaskRow {
    pub id: String,
    pub run_id: String,
    pub agent_kind: String,
    pub description: String,
    pub status: String,
    pub parent_task_id: Option<String>,
    pub created_at: String,
}
