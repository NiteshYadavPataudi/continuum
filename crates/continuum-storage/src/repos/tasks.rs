use sqlx::SqlitePool;

use crate::error::StorageError;

/// Repository for managing tasks within a run.
pub struct TaskRepo {
    pool: SqlitePool,
}

impl TaskRepo {
    /// Create a new `TaskRepo` backed by the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new task.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
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

    /// Retrieve a task by its id.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn get(&self, id: &str) -> Result<Option<TaskRow>, StorageError> {
        let row = sqlx::query_as::<_, TaskRow>(
            "SELECT id, run_id, agent_kind, description, status, parent_task_id, created_at FROM tasks WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// List all tasks for a run, ordered by creation time.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
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

/// A row representing a single task within a run.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TaskRow {
    /// Unique task identifier.
    pub id: String,
    /// The run this task belongs to.
    pub run_id: String,
    /// The kind of agent assigned to this task.
    pub agent_kind: String,
    /// A description of what the task entails.
    pub description: String,
    /// Current task status (e.g. "pending", "in_progress", "completed").
    pub status: String,
    /// Optional id of the parent task (for subtask hierarchies).
    pub parent_task_id: Option<String>,
    /// ISO-8601 timestamp of when the task was created.
    pub created_at: String,
}
